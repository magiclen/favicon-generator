use std::ffi::OsStr;
use std::io::{self, ErrorKind, Write};
use std::process::{Command, Stdio};

#[inline]
fn unexpected_exit_status() -> io::Error {
    io::Error::new(ErrorKind::Other, "unexpected exit status")
}

#[inline]
fn generate_command<P, I, S>(program: P, args: I) -> Result<Command, io::Error>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>, {
    let mut command = Command::new(program.as_ref());

    for arg in args {
        command.arg(arg);
    }

    Ok(command)
}

#[inline]
pub fn check_executable<P, I, S>(
    program: P,
    args: I,
    expect_exit_status: i32,
) -> Result<(), io::Error>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>, {
    let mut command = generate_command(program, args)?;

    command.stdout(Stdio::null());
    command.stderr(Stdio::null());

    let status = command.status()?;

    match status.code() {
        Some(code) if code == expect_exit_status => Ok(()),
        _ => Err(unexpected_exit_status()),
    }
}

#[inline]
pub fn execute_one_stdin<P, I, S, D>(program: P, args: I, input: &D) -> Result<i32, io::Error>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    D: ?Sized + AsRef<[u8]>, {
    let mut command = generate_command(program, args)?;

    command.stdin(Stdio::piped());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());

    let mut child = command.spawn()?;

    {
        let stdin = child.stdin.as_mut().unwrap();

        stdin.write_all(input.as_ref())?;
    }

    Ok(child.wait()?.code().ok_or_else(unexpected_exit_status)?)
}
