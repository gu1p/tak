fn respond_with_node_info(listener: &TcpListener, body: &[u8]) {
    let (mut stream, _) = listener.accept().expect("accept node info");
    let request = read_request_head(&mut stream);
    assert!(
        request.starts_with("GET /v1/node/info HTTP/1.1\r\n"),
        "unexpected request: {request}"
    );
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("write node info head");
    stream.write_all(body).expect("write node info body");
}

fn respond_with_optional_status_then_submit_auth_failure(
    listener: &TcpListener,
    status_body: &[u8],
) {
    let (mut stream, _) = listener.accept().expect("accept submit");
    let request = read_request_head(&mut stream);
    if request.starts_with("GET /v1/node/status HTTP/1.1\r\n") {
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            status_body.len()
        )
        .expect("write status head");
        stream.write_all(status_body).expect("write status body");
        return respond_with_upload_negotiation_then_submit_auth_failure(listener);
    }
    respond_with_upload_negotiation_or_submit_auth_failure(listener, stream, request);
}

fn respond_with_submit_auth_failure(listener: &TcpListener) {
    let (stream, _) = listener.accept().expect("accept submit");
    let mut stream = stream;
    let request = read_request_head(&mut stream);
    respond_with_submit_auth_failure_request(stream, request);
}

fn respond_with_upload_negotiation_then_submit_auth_failure(listener: &TcpListener) {
    let (stream, _) = listener.accept().expect("accept upload negotiation");
    let mut stream = stream;
    let request = read_request_head(&mut stream);
    respond_with_upload_negotiation_or_submit_auth_failure(listener, stream, request);
}

fn respond_with_upload_negotiation_or_submit_auth_failure(
    listener: &TcpListener,
    mut stream: impl Write,
    request: String,
) {
    if request.starts_with("POST /v2/workspaces/uploads/begin HTTP/1.1\r\n") {
        write!(
            stream,
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .expect("write upload negotiation response");
        return respond_with_submit_auth_failure(listener);
    }
    respond_with_submit_auth_failure_request(stream, request);
}

fn respond_with_submit_auth_failure_request(mut stream: impl Write, request: String) {
    assert!(
        request.starts_with("POST /v1/tasks/submit HTTP/1.1\r\n"),
        "unexpected request: {request}"
    );
    write!(
        stream,
        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
    .expect("write submit response");
}

fn read_request_head(stream: &mut impl Read) -> String {
    let mut request = Vec::new();
    let mut buf = [0_u8; 256];
    loop {
        let read = stream.read(&mut buf).expect("read request");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buf[..read]);
        if let Some(index) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            request.truncate(index + 4);
            break;
        }
    }
    String::from_utf8(request).expect("request head utf8")
}
