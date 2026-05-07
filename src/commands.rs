/// Context key for user-defined command widgets. Value = widget name.
pub const CMD_CONTEXT_USER: &str = "cmd_widget";

/// Output captured from a command widget execution.
#[derive(Clone, Default)]
pub struct CommandOutput {
    pub stdout: String,
    pub exit_code: i32,
}
