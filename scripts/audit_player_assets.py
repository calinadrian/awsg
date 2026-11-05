"""Verify a packaged MPQ against its manifest and distinguish missing source references.

Uses the runtime's MPQ reader; does not launch the game or modify the source Data folder.
"""
import argparse
import hashlib
import json
from pathlib import Path

from package_elwynn import Reader


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    parser.add_argument('--source', type=Path, required=True)
    args = parser.parse_args()
    manifest = json.loads((args.package / 'asset-manifest.json').read_text())
    packed, source = Reader(args.package / 'Data'), Reader(args.source)
    failures = []
    try:
        for entry in manifest['files']:
            raw = packed.read(entry['path'], optional=True)
            if raw is None or len(raw) != entry['bytes'] or hashlib.sha256(raw).hexdigest() != entry['sha256']:
                failures.append(entry['path'])
        for entry in manifest['source_missing']:
            path = entry.get('path')
            if path and ':' not in path and '..' not in path.replace('/', '\\').split('\\'):
                if source.read(path, optional=True) is not None:
                    failures.append('Available source asset omitted: ' + path)
            elif 'display' in entry:
                failures.append('Missing required creature display: ' + str(entry['display']))
    finally:
        packed.close()
        source.close()
    report = dict(verified_assets=len(manifest['files']), failures=failures,
                  absent_source_references=manifest['source_missing'], gameplay_tested=False)
    (args.package / 'asset-audit.json').write_text(json.dumps(report, indent=2) + '\n')
    if failures:
        raise SystemExit('\n'.join(failures))
    print(f"Verified {len(manifest['files']):,} packaged asset sizes and SHA-256 hashes; "
          f"{len(manifest['source_missing'])} references absent from source data.")


if __name__ == '__main__':
    main()
