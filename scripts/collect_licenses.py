"""Record the locked Windows build/test dependency graph and copy root notices.

Run after cargo metadata --locked --format-version 1 --filter-platform
x86_64-pc-windows-msvc > .research/cargo-metadata.json.
This is evidence collection, not an assertion of patent or license clearance.
"""
import hashlib
import json
import shutil
from pathlib import Path

root = Path(__file__).resolve().parents[1]
metadata = json.loads((root / '.research/cargo-metadata.json').read_text(encoding='utf-8-sig'))
nodes = {n['id']: n for n in metadata['resolve']['nodes']}
active = set()
pending = list(metadata['workspace_members'])
while pending:
    key = pending.pop()
    if key in active:
        continue
    active.add(key)
    pending.extend(d['pkg'] for d in nodes[key]['deps'])
records = []
for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
    if package['id'] not in active or package['id'] in metadata['workspace_members']:
        continue
    folder = Path(package['manifest_path']).parent
    files = [p for p in folder.iterdir() if p.is_file() and p.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'PATENTS'))]
    if package.get('license_file'):
        files.append(folder / package['license_file'])
    if package['name'] == 'fdk-aac-sys':
        files.append(folder / 'aac/NOTICE')
    if package['name'] == 'openh264-sys2':
        files.append(folder / 'upstream/LICENSE')
    # GPUI's workspace crates share license texts at repository root.
    if 'gpui-fusion' in (package.get('source') or ''):
        repo = folder
        while repo.parent != repo and not (repo / '.git').exists():
            repo = repo.parent
        if (repo / '.git').exists():
            files.extend(p for p in repo.iterdir() if p.is_file() and p.name.upper().startswith(('LICENSE', 'LICENCE', 'NOTICE')))
    copied = []
    # These published dasp subcrates omit the workspace's license file.
    # We retrieved it from the exact .cargo_vcs_info.json commit; SOURCE.txt
    # beside the notice records provenance. Preserve this supplemental text.
    if package['name'] in ('dasp_sample', 'dasp_frame'):
        supplement = root / 'licenses/third-party' / f'{package["name"]}-{package["version"]}' / 'LICENSE-MIT'
        if supplement.is_file():
            copied.append({'path': supplement.relative_to(root).as_posix(), 'sha256': hashlib.sha256(supplement.read_bytes()).hexdigest()})
    for source in sorted(set(files)):
        if not source.is_file():
            continue
        name = str(source.relative_to(folder)).replace('..', 'parent').replace('\\', '_').replace('/', '_') if source.is_relative_to(folder) else 'workspace_' + source.name
        destination = root / 'licenses/third-party' / f'{package["name"]}-{package["version"]}' / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        copied.append({'path': destination.relative_to(root).as_posix(), 'sha256': hashlib.sha256(destination.read_bytes()).hexdigest()})
    records.append({'name': package['name'], 'version': package['version'], 'license_metadata': package['license'], 'source': package.get('source'), 'repository': package.get('repository'), 'notices': copied})
out = root / 'docs/research/dependency-licenses.json'
out.write_text(json.dumps({'scope': 'Cargo.lock resolved Windows build/test graph; other targets and release packaging require separate review', 'packages': records}, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(f'Recorded {len(records)} packages; {sum(len(r["notices"]) for r in records)} notice files')
print('No root license text:', ', '.join(r['name'] for r in records if not r['notices']))
