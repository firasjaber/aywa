use std::{ffi::OsStr, process::Command};

#[cfg(target_os = "windows")]
pub const INTERNAL_COMMAND_CREATION_FLAGS: u32 = 0x0800_0000;

pub fn internal_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        command.creation_flags(INTERNAL_COMMAND_CREATION_FLAGS);
    }

    command
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    use super::INTERNAL_COMMAND_CREATION_FLAGS;

    #[cfg(target_os = "windows")]
    #[test]
    fn internal_commands_use_the_no_console_creation_flag() {
        assert_eq!(INTERNAL_COMMAND_CREATION_FLAGS, 0x0800_0000);
    }
}
