import argparse
import os.path


# Usage example (called from the project root).
# To build WebAssembly and copy all necessary files to a dist folder:
#   python3 crates/wasm-server/compose.py compose --dist crates/wasm-server/dist
# To start a local server to serve the dist folder:
#   python3 crates/wasm-server/compose.py serve --dist crates/wasm-server/dist --port 8080


def run(args: list[str], allow_failure: bool = False) -> int:
    import subprocess
    result = subprocess.run(args)
    if result.returncode != 0 and not allow_failure:
        raise RuntimeError(f"Command {' '.join(args)} failed with exit code {result.returncode}")
    return result.returncode

def compose(dist_path: str, dev: bool = False):
    # Step 1. Build the WebAssembly project using `wasm-pack`.
    run(['wasm-pack', 'build', 'crates/wasm', '--out-dir', os.path.join(dist_path, 'pkg'), '--target', 'web'] + (['--dev'] if dev else ['--release']))
    # Step 2. Build the Assets
    run(['cargo', 'run', '--bin', 'dawn-package', '--', '--assets-dir', 'assets', '--output-file', os.path.join(dist_path, 'assets.dac')])
    pass

def serve(dist_path: str, port: int):
    run(['cargo', 'run', '--bin', 'dawn-wasm-server', '--', '--dist', dist_path, '--port', str(port)])

def main():
    parser = argparse.ArgumentParser(description='Compose a DAWN WebAssembly server project.')
    subparsers = parser.add_subparsers(dest='command')
    subparsers.required = True
    compose_parser = subparsers.add_parser('compose', help='Build and compose the project.')
    compose_parser.add_argument('--dist', type=str, required=True, help='Path to the distribution folder.')
    compose_parser.add_argument('--dev', action='store_true', help='Use development mode (default is release mode).')

    serve_parser = subparsers.add_parser('serve', help='Start a local server to serve the dist folder.')
    serve_parser.add_argument('--dist', type=str, required=True, help='Path to the distribution folder.')
    serve_parser.add_argument('--port', type=int, default=8000, help='Port to serve on (default: 8000).')

    args = parser.parse_args()

    # Make paths absolute
    import os
    args.dist = os.path.abspath(args.dist)

    if args.command == 'compose':
        compose(args.dist, args.dev)
    elif args.command == 'serve':
        serve(args.dist, args.port)
    else:
        parser.print_help()

if __name__ == '__main__':
    main()