"""Explicit installed-backend characterization; no production qualification."""
import json
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[6]/'tools'))
import wdbx_complete_p1_crypto as p1
vector=json.loads(Path(__file__).with_name('ph-vector.json').read_text())
public=bytes.fromhex(vector['public_key_hex'])
structure=bytes.fromhex(vector['sig_structure_hex'])
signature=p1._sign(bytes.fromhex(vector['synthetic_seed_hex']),structure)
order=2**252+27742317777372353535851937790883648493
cases={
 'ordinary_positive':signature,
 'invalid_R':bytes([255])*32+signature[32:],
 'identity_R':b'\x01'+bytes(31)+signature[32:],
 'noncanonical_S':signature[:32]+(int.from_bytes(signature[32:],'little')+order).to_bytes(32,'little'),
 'native_ed25519ph_confusion':bytes.fromhex(vector['signature_hex']),
}
print(json.dumps({'backend_probe':p1.backend_probe(),'raw_signature_outcomes':{name:p1._verify(public,sig,structure) for name,sig in cases.items()},'full_p1_ph_result':p1.request(json.dumps({'op':'verify','container_hex':vector['container_hex'],'payload_hex':vector['payload_hex'],'public_key_hex':vector['public_key_hex']}).encode())},indent=2,sort_keys=True))
