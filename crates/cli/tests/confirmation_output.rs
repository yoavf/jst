use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::thread;

#[test]
fn risky_confirmation_shows_the_command_when_stdout_is_captured() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let address = listener.local_addr().expect("test server address");
    let server = thread::spawn(move || {
        let (mut connection, _) = listener.accept().expect("accept CLI request");
        let mut request = [0_u8; 4096];
        let bytes_read = connection.read(&mut request).expect("read CLI request");
        assert!(bytes_read > 0, "CLI request was empty");

        let body = r#"{"command":"rm -rf build","effects":{"reads_data":false,"modifies_data":true,"deletes_data":true,"uses_network":false,"changes_remote_data":false,"changes_processes":false,"installs_software":false,"uses_privilege":false,"executes_remote_code":false},"matches_request":true,"explanation":"Removes the build directory."}"#;
        write!(
            connection,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("send translation response");
    });

    let mut child = Command::new(env!("CARGO_BIN_EXE_jst"))
        .args(["remove", "build"])
        .env("JST_API_URL", format!("http://{address}/translate"))
        .env(
            "JST_INSTALLATION_ID",
            "00000000-0000-4000-8000-000000000000",
        )
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start CLI");
    child
        .stdin
        .take()
        .expect("CLI stdin")
        .write_all(b"n\n")
        .expect("decline confirmation");

    let output = child.wait_with_output().expect("collect CLI output");
    server.join().expect("test server");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");

    assert!(
        stderr.contains("→ rm -rf build"),
        "confirmation UI omitted the command:\n{stderr}"
    );
    assert!(stderr.contains("This command deletes files or directories."));
    assert!(stderr.contains("Run it? [y/N]"));
    assert!(
        !stdout.contains("rm -rf build"),
        "confirmation command escaped to captured stdout:\n{stdout}"
    );
}
