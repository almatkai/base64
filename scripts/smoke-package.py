#!/usr/bin/env python3
"""Verify a target ZIP and drive its extracted executable offline over stdio.

Acts as a fake narrow host: answers typed host.* reverse RPCs from a script and
records them. Never installs the package or touches the real clipboard/window."""
import argparse, hashlib, json, os, queue, stat, subprocess, tempfile, threading, uuid, zipfile
from pathlib import Path, PurePosixPath

CLIPBOARD = 'aGk='  # Base64 of "hi"


def smoke(path):
    with tempfile.TemporaryDirectory() as temp, zipfile.ZipFile(path) as archive:
        root = Path(temp) / 'package'
        root.mkdir()
        names = archive.namelist()
        assert len(names) == len(set(names)), 'Duplicate ZIP paths'
        for item in archive.infolist():
            name = PurePosixPath(item.filename)
            assert not name.is_absolute() and '..' not in name.parts and '\\' not in item.filename and ':' not in item.filename, 'Unsafe ZIP path'
            assert not stat.S_ISLNK(item.external_attr >> 16), 'Symlink ZIP entry'
        manifest = json.loads(archive.read('extension.json'))
        inventory = manifest['files']
        assert set(names) == set(inventory) | {'extension.json'}, 'ZIP/inventory mismatch'
        for name, digest in inventory.items():
            assert hashlib.sha256(archive.read(name)).hexdigest() == digest, 'SHA inventory mismatch: ' + name
        for required in ['LICENSE', 'NOTICE', manifest['icon']]:
            assert required in inventory, 'Missing ' + required
        archive.extractall(root)
        slug = manifest['slug']
        binary = root / 'bin' / ('tezbar-' + slug + ('.exe' if os.name == 'nt' else ''))
        assert os.name == 'nt' or (archive.getinfo(binary.relative_to(root).as_posix()).external_attr >> 16) & 0o111, 'Executable bit missing'
        binary.chmod(0o755)
        env = {k: v for k, v in os.environ.items() if k in ('SystemRoot', 'WINDIR', 'TEMP', 'TMP')}
        env['PATH'] = os.path.join(env.get('SystemRoot', 'C:/Windows'), 'System32') if os.name == 'nt' else '/usr/bin:/bin'
        info = json.loads(subprocess.check_output([str(binary), '--protocol-info'], cwd=root, env=env, timeout=10))
        assert info['id'] == manifest['id'] and info['version'] == manifest['version'] and info['protocol'] == 1
        assert info['commands'] == [c['name'] for c in manifest['commands']]
        process = subprocess.Popen([str(binary)], cwd=root, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=None)
        messages = queue.Queue()
        calls = []
        granted = set(manifest['capabilities'])

        def reader():
            try:
                while True:
                    line = process.stdout.readline(16 * 1024 * 1024 + 1)
                    if not line:
                        messages.put(EOFError('Unexpected EOF'))
                        return
                    if len(line) > 16 * 1024 * 1024:
                        raise ValueError('Oversized frame')
                    messages.put(json.loads(line))
            except Exception as error:
                messages.put(error)

        threading.Thread(target=reader, daemon=True).start()

        def receive():
            value = messages.get(timeout=8)
            if isinstance(value, Exception):
                raise value
            return value

        def send(value):
            process.stdin.write((json.dumps(value) + '\n').encode())
            process.stdin.flush()

        def request(method, params, expect_error=False):
            ident = 'h:' + str(uuid.uuid4())
            send({'type': 'request', 'id': ident, 'method': method, 'params': params})
            while True:
                frame = receive()
                if frame['type'] == 'response' and frame['id'] == ident:
                    if expect_error:
                        assert 'error' in frame, frame
                        return frame['error']
                    assert 'error' not in frame, frame
                    return frame['result']
                if frame['type'] == 'request':
                    method_name = frame['method']
                    assert frame['id'].startswith('e:') and method_name.startswith('host.'), frame
                    assert method_name.removeprefix('host.') in granted, 'Ungranted RPC ' + method_name
                    calls.append((method_name, frame['params']))
                    result = CLIPBOARD if method_name == 'host.clipboard.read' else None
                    send({'type': 'response', 'id': frame['id'], 'result': result})
                    continue
                assert frame['type'] == 'event', frame

        try:
            hello = receive()
            assert hello['type'] == 'hello' and hello['id'] == manifest['id'] and hello['version'] == manifest['version'] and hello['protocol'] == 1
            ready = request('initialize', {'protocol': 1, 'extensionId': manifest['id'], 'platform': 'Windows' if os.name == 'nt' else 'macOS', 'packageDir': str(root), 'dataDir': str(Path(temp) / 'data'), 'tempDir': str(Path(temp) / 'temp'), 'grantedCapabilities': manifest['capabilities'], 'reason': 'command', 'migrationPaths': {}})
            assert ready['ready'] is True
            view = request('session.open', {'sessionId': 'zip-smoke', 'commandName': 'index', 'preferences': {}, 'argumentValues': {}})
            assert view['mode'] == 'view' and view['root']['type'] == 'List', view
            subtitles = [row['props']['subtitle'] for row in view['root']['children'][0]['children']]
            assert subtitles == ['YUdrPQ==', 'hi'], subtitles
            copy = next(a['id'] for a in view['actions'] if a['id'].endswith(':decode:copyToClipboard'))
            after = request('session.call', {'sessionId': 'zip-smoke', 'operation': 'extension:invoke-action', 'payload': {'actionId': copy}})
            assert after['mode'] == 'view'
            assert calls[-1] == ('host.clipboard.write', {'text': 'hi'}), calls
            stale = request('session.call', {'sessionId': 'zip-smoke', 'operation': 'extension:invoke-action', 'payload': {'actionId': 'base64:999:decode:copyToClipboard'}}, expect_error=True)
            assert 'stale' in stale['message'], stale
            request('session.dispose', {'sessionId': 'zip-smoke'})
            result = request('session.open', {'sessionId': 'zip-encode', 'commandName': 'encode', 'preferences': {}, 'argumentValues': {}})
            assert result['mode'] == 'no-view', result
            assert calls[-2:] == [('host.clipboard.write', {'text': 'YUdrPQ=='}), ('host.window.visibility', {'visible': False})], calls
            request('session.dispose', {'sessionId': 'zip-encode'})
            request('shutdown', {})
            assert process.wait(timeout=5) == 0, 'Process did not exit cleanly'
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
            process.stdin.close()
            process.stdout.close()
        print(slug, f'ZIP inventory ({len(inventory)} files) + protocol-info/hello/initialize/open/search-view/copy-action/stale-reject/encode/dispose/shutdown passed with {len(calls)} recorded fake-host RPCs (offline, no installation)')


if __name__ == '__main__':
    p = argparse.ArgumentParser()
    p.add_argument('archive', type=Path)
    smoke(p.parse_args().archive)
