use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyDict, PyList};
use std::collections::HashMap;

#[pyclass(frozen)]
pub struct DiffRow {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub baseline_mean_ns: Option<f64>,
    #[pyo3(get)]
    pub current_mean_ns: Option<f64>,
    #[pyo3(get)]
    pub change_pct: Option<f64>,
    #[pyo3(get)]
    pub classification: String,
}

#[pyclass(frozen)]
pub struct ComparisonReport {
    #[pyo3(get)]
    pub rows: Vec<Py<DiffRow>>,
}

#[pymethods]
impl ComparisonReport {
    fn format(&self, py: Python<'_>, fmt: &str) -> PyResult<String> {
        let rows: Vec<DiffRowView> = self
            .rows
            .iter()
            .map(|r| {
                let r = r.borrow(py);
                DiffRowView {
                    name: r.name.clone(),
                    baseline_mean_ns: r.baseline_mean_ns,
                    current_mean_ns: r.current_mean_ns,
                    change_pct: r.change_pct,
                    classification: r.classification.clone(),
                }
            })
            .collect();
        match fmt {
            "table" => Ok(crate::report::format_comparison_table(&rows)),
            "json" => Ok(crate::report::format_comparison_json(&rows)),
            "html" => Ok(crate::report::format_comparison_html(&rows)),
            "xml" => Ok(crate::report::format_comparison_xml(&rows)),
            other => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "unknown format: {other}"
            ))),
        }
    }
}

pub struct DiffRowView {
    pub name: String,
    pub baseline_mean_ns: Option<f64>,
    pub current_mean_ns: Option<f64>,
    pub change_pct: Option<f64>,
    pub classification: String,
}

struct ResultRow {
    name: String,
    mean_ns: f64,
    ci_low: f64,
    ci_high: f64,
}

#[pyfunction]
pub fn compare(
    py: Python<'_>,
    baseline_json: &str,
    current_json: &str,
) -> PyResult<ComparisonReport> {
    let json_mod = py.import_bound("json")?;
    let loads = json_mod.getattr("loads")?;
    let baseline = loads
        .call1((baseline_json,))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("baseline: {e}")))?;
    let current = loads
        .call1((current_json,))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("current: {e}")))?;
    let b_rows = extract_rows(&baseline)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("baseline: {e}")))?;
    let c_rows = extract_rows(&current)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("current: {e}")))?;
    let mut by_name: HashMap<String, ResultRow> =
        c_rows.into_iter().map(|r| (r.name.clone(), r)).collect();
    let mut out: Vec<Py<DiffRow>> = Vec::new();
    for b in b_rows {
        let row = match by_name.remove(&b.name) {
            None => DiffRow {
                name: b.name,
                baseline_mean_ns: Some(b.mean_ns),
                current_mean_ns: None,
                change_pct: None,
                classification: "removed".into(),
            },
            Some(c) => {
                let pct = (c.mean_ns - b.mean_ns) / b.mean_ns * 100.0;
                let cls = if c.ci_low > b.ci_high {
                    "regressed"
                } else if c.ci_high < b.ci_low {
                    "improved"
                } else {
                    "unchanged"
                };
                DiffRow {
                    name: b.name,
                    baseline_mean_ns: Some(b.mean_ns),
                    current_mean_ns: Some(c.mean_ns),
                    change_pct: Some(pct),
                    classification: cls.into(),
                }
            }
        };
        out.push(Py::new(py, row)?);
    }
    for (_, c) in by_name {
        out.push(Py::new(
            py,
            DiffRow {
                name: c.name,
                baseline_mean_ns: None,
                current_mean_ns: Some(c.mean_ns),
                change_pct: None,
                classification: "new".into(),
            },
        )?);
    }
    Ok(ComparisonReport { rows: out })
}

fn extract_rows(payload: &Bound<'_, PyAny>) -> PyResult<Vec<ResultRow>> {
    let results = payload
        .downcast::<PyDict>()
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("payload is not a JSON object"))?
        .get_item("results")?
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("missing 'results' key"))?;
    let arr: Bound<'_, PyList> = results
        .downcast_into()
        .map_err(|_| pyo3::exceptions::PyValueError::new_err("'results' is not an array"))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr.iter() {
        let d: Bound<'_, PyDict> = item
            .downcast_into()
            .map_err(|_| pyo3::exceptions::PyValueError::new_err("result row is not an object"))?;
        let name: String = d
            .get_item("name")?
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("missing 'name'"))?
            .extract()?;
        let mean_ns: f64 = d
            .get_item("mean_ns")?
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("missing 'mean_ns'"))?
            .extract()?;
        let ci_low: f64 = d
            .get_item("ci95_low_ns")?
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("missing 'ci95_low_ns'"))?
            .extract()?;
        let ci_high: f64 = d
            .get_item("ci95_high_ns")?
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("missing 'ci95_high_ns'"))?
            .extract()?;
        out.push(ResultRow {
            name,
            mean_ns,
            ci_low,
            ci_high,
        });
    }
    Ok(out)
}
