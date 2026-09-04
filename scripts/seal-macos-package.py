#!/usr/bin/env python3
"""Seal local macOS bundles ad hoc, then create a portable DMG outside FileProvider metadata."""
import platform,subprocess,tempfile
from pathlib import Path
if platform.system()!='Darwin':raise SystemExit('macOS packaging only')
root=Path(__file__).resolve().parents[1]
source=root/'target/release/bundle/macos/WDBX Specimen Studio.app'
images=list((root/'target/release/bundle/dmg').glob('*.dmg'))
if len(images)!=1:raise SystemExit('Build exactly one macOS target before sealing')
with tempfile.TemporaryDirectory(prefix='wdbx-package-') as temporary:
 stage=Path(temporary);app=stage/source.name
 subprocess.run(['ditto','--noextattr','--norsrc',str(source),str(app)],check=True)
 for name in ['llama-server','sd-cli']:
  subprocess.run(['codesign','--force','--sign','-',str(app/'Contents/Resources/binaries'/name)],check=True)
 subprocess.run(['codesign','--force','--sign','-',str(app)],check=True)
 subprocess.run(['codesign','--verify','--deep','--strict','--verbose=2',str(app)],check=True)
 (stage/'Applications').symlink_to('/Applications')
 subprocess.run(['hdiutil','create','-ov','-volname','WDBX Specimen Studio','-srcfolder',str(stage),'-format','UDZO',str(images[0])],check=True)
 subprocess.run(['hdiutil','verify',str(images[0])],check=True)
print('Ad-hoc bundle sealing and DMG integrity passed. Developer ID notarization is separate.')
