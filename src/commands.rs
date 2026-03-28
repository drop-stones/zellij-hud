/// Context key for user-defined command widgets. Value = widget name.
pub(crate) const CMD_CONTEXT_USER: &str = "cmd_widget";

/// Output captured from a command widget execution.
#[derive(Clone, Default)]
pub(crate) struct CommandOutput {
    pub(crate) stdout: String,
    pub(crate) exit_code: i32,
}
