import os, shutil, subprocess

def run(cmd):
    print(cmd)
    subprocess.run(cmd, shell=True, check=True)

version = None
with open('artefact-cli/Cargo.toml') as f:
    for line in f:
        if 'version' in line:
            version = line.split('"')[1]
            break

def build(target, arch):
    run(f'rustup target add {target}')
    run(f'cargo build --bin artefact-cli --release --target {target}')
    if "windows" in target:
        exe = 'artefact-cli.exe'
    else:
        exe = 'artefact-cli'
    if not os.path.exists(f'target/{target}/release/{exe}'):
        raise Exception(f'Build failed for {target}')

    ext = '.zip' if "windows" in target else '.tar.gz'
    pkg = 'zip -j' if ext == '.zip' else 'tar -czvf'
    ver = f'-{version}' if version is not None else ''
    run(f'{pkg} dist-cli/artefact-cli{ver}-{arch}{ext} target/{target}/release/{exe}')

shutil.rmtree('dist-cli', ignore_errors=True)
os.makedirs('dist-cli', exist_ok=True)

build('i686-pc-windows-gnu', 'win-32')
build('x86_64-pc-windows-gnu', 'win-64')
build('i686-unknown-linux-gnu', 'linux-32')
build('x86_64-unknown-linux-gnu', 'linux-64')