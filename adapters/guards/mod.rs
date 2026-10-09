//! Admission uses native GE bindings and controller source pins; no specialist
//! analysis or authority algorithm is implemented by this adapter.
use guardengine::integration::RunBinding;
pub(crate) fn expected_binding(
    controller: &RunBinding,
    sources: Option<&crate::evidence::ScopedSources>,
    scope: &str,
) -> Result<RunBinding, &'static str> {
    if let Some(sources) = sources {
        return sources.expected_binding(controller, scope);
    }
    Ok(controller.clone())
}
