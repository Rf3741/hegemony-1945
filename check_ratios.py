import subprocess
import re
from collections import defaultdict

# Run proto_sim3 logic and print provinces with high vagas / pop
import proto_sim3

print(f"{'Provincia':25} {'Pop':>10} {'Vagas':>10} {'Ratio':>8}")
for p in sorted(proto_sim3.provs, key=lambda x: proto_sim3.prov_levels[x['id']]['mina_ferro'] + proto_sim3.prov_levels[x['id']]['mina_carvao'] + proto_sim3.prov_levels[x['id']]['fazenda_graos'], reverse=True)[:15]:
    pop_p = proto_sim3.cur_pop_by_id[p['id']]
    vagas_p = sum(proto_sim3.prov_levels[p['id']][t] * proto_sim3.vagas_ratio.get(t, 450.0) for t in proto_sim3.prov_levels[p['id']])
    ratio = vagas_p / pop_p if pop_p > 0 else 0
    print(f"{p['nome']:25} {pop_p:10.0f} {vagas_p:10.0f} {ratio:8.2f}")
