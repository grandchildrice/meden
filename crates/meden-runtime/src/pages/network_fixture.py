"""Disposable DNS/TLS receivers, reachable only inside the test's network namespace."""

import ipaddress
import json
import socket
import ssl
import struct
import sys
import threading
from pathlib import Path

PUBLIC4 = "93.184.216.34"
PUBLIC6 = "2606:4700:4700::1111"
state = {"dns": [], "http": [], "connections": [], "controls": [], "tls_errors": []}
lock = threading.Lock()
rebind = 0


def record(kind, value):
    with lock:
        state[kind].append(value)


def dns():
    global rebind
    receiver = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    receiver.bind(("127.0.0.1", 53))
    ready.set()
    while True:
        data, peer = receiver.recvfrom(4096)
        cursor = 12
        labels = []
        while data[cursor]:
            size = data[cursor]
            cursor += 1
            labels.append(data[cursor:cursor + size].decode("ascii"))
            cursor += size
        cursor += 1
        name = ".".join(labels)
        kind, klass = struct.unpack("!HH", data[cursor:cursor + 4])
        question = data[12:cursor + 4]
        addresses = []
        if kind == 1:
            if name == "rebind.meden.test":
                rebind += 1
                addresses = [PUBLIC4 if rebind == 1 else "127.0.0.1"]
            elif name == "mixed.meden.test":
                addresses = [PUBLIC4, "127.0.0.1"]
            elif name == "private.meden.test":
                addresses = ["127.0.0.1"]
            elif name == "metadata.meden.test":
                addresses = ["169.254.169.254"]
            elif name != "v6.meden.test" and name != "mapped.meden.test":
                addresses = [PUBLIC4]
        elif kind == 28:
            if name == "v6.meden.test":
                addresses = [PUBLIC6]
            elif name == "mapped.meden.test":
                addresses = ["::ffff:127.0.0.1"]
        record("dns", {"name": name, "type": kind, "answers": addresses})
        answers = b"".join(
            b"\xc0\x0c" + struct.pack("!HHIH", kind, klass, 0, len(packed)) + packed
            for packed in [ipaddress.ip_address(address).packed for address in addresses]
        )
        receiver.sendto(
            data[:2] + struct.pack("!HHHHH", 0x8180, 1, len(addresses), 0, 0)
            + question + answers, peer,
        )


def serve(listener, role):
    while True:
        stream, _ = listener.accept()
        record("connections", role)
        stream.settimeout(3)
        try:
            with stream:
                if role not in ["public4", "public6"]:
                    data = stream.recv(8192)
                    record("controls", {"role": role, "body": data.decode(errors="replace"),
                                        "body_hex": data.hex()})
                    stream.sendall(b"ack")
                    continue
                with context.wrap_socket(stream, server_side=True) as channel:
                    request = b""
                    while b"\r\n\r\n" not in request:
                        piece = channel.recv(4096)
                        if not piece or len(request) > 16384:
                            raise ValueError("incomplete or oversized fixture request")
                        request += piece
                    text = request.decode()
                    record("http", {"role": role, "request": text})
                    path = text.split(" ")[1]
                    status, extra = "200 OK", ""
                    body = b"<script>fetch('https://subresource.meden.test/secret')</script>"
                    if path.startswith("/redirect/"):
                        host = path.removeprefix("/redirect/")
                        status = "307 Temporary Redirect"
                        extra = f"Location: https://{host}:{port}/unapproved\r\n"
                        body = b""
                    headers = (f"HTTP/1.1 {status}\r\n{extra}Content-Type: text/html\r\n"
                               f"Content-Length: {len(body)}\r\nConnection: close\r\n\r\n")
                    channel.sendall(headers.encode() + body)
        except (ssl.SSLError, ConnectionError, TimeoutError) as error:
            record("tls_errors", type(error).__name__)


fixtures = Path(__file__).with_name("testdata")
context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.load_cert_chain(fixtures / "server.pem", fixtures / "fixture-key.pem")
listeners = []
port = int(sys.argv[1])
for host, role in [
    (PUBLIC4, "public4"), (PUBLIC6, "public6"), ("127.0.0.1", "private"),
    ("169.254.169.254", "metadata"), ("::1", "ipv6-private"),
]:
    family = socket.AF_INET6 if ":" in host else socket.AF_INET
    receiver = socket.socket(family, socket.SOCK_STREAM)
    if family == socket.AF_INET6:
        receiver.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 1)
    receiver.bind((host, port))
    port = receiver.getsockname()[1]
    receiver.listen()
    listeners.append(receiver)
    threading.Thread(target=serve, args=(receiver, role), daemon=True).start()
ready = threading.Event()
threading.Thread(target=dns, daemon=True).start()
assert ready.wait(3), "fixture DNS did not start"
print(json.dumps({"port": port}), flush=True)
for line in sys.stdin:
    assert line.strip() == "snapshot"
    with lock:
        print(json.dumps(state), flush=True)
