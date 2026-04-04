pub(crate) fn get_function_tool_task_progress_deadline(
    timeout_seconds: Option<f64>,
) -> Option<tokio::time::Instant> {
    timeout_seconds.map(|seconds| {
        tokio::time::Instant::now() + std::time::Duration::from_secs_f64(seconds.max(0.0))
    })
}
