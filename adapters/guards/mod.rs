//! Admission uses native GE bindings and controller source pins; no specialist
//! analysis or authority algorithm is implemented by this adapter.
use guardengine::integration::RunBinding;
pub(crate) fn expected_binding(
    controller: &RunBinding,
    sources: Option<&crate::evidence::ScopedSources>,
    scope: &str,
) -> Result<RunBinding, &'static str> {
    let mut expected = controller.clone();
    if let Some(sources) = sources {
        expected.source_snapshot_digest = sources
            .source(scope)
            .ok_or("missing specialist source pin")?
            .into();
    }
    Ok(expected)
}
