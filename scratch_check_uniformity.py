import os
import subprocess
import re
from collections import defaultdict

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

for country in ["bra", "sov", "ger"]:
    b_sets = []
    for line in j_text.split("\n"):
        if f'pais_id:"{country}"' in line:
            m_pred = re.search(r'predios:\[(.*)\]', line)
            if m_pred:
                bs = tuple(sorted(re.findall(r'tipo_predio_id:"([^"]+)",nivel:(\d+)', m_pred.group(1))))
                b_sets.append(bs)
    unique_sets = set(b_sets)
    print(f"{country}: total provs={len(b_sets)}, unique building distributions={len(unique_sets)}")
