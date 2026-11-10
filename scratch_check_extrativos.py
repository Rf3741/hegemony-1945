import os
import subprocess
import re
from collections import Counter

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

extrativo_types = {
    "mina_ferro", "mina_carvao", "poco_petroleo", "campo_gas_natural",
    "mina_cobre", "mina_uranio", "mina_bauxita", "porto_pesca"
}

extrativo_count = 0
non_extrativo_count = 0
by_type = Counter()

for line in j_text.split("\n"):
    if 'tipo_predio_id:' in line:
        for m in re.finditer(r'tipo_predio_id:"([^"]+)"', line):
            tp = m.group(1)
            by_type[tp] += 1
            if tp in extrativo_types:
                extrativo_count += 1
            else:
                non_extrativo_count += 1

print(f"Total buildings in jogavel: {extrativo_count + non_extrativo_count}")
print(f"Extrativo buildings: {extrativo_count}")
print(f"Non-extrativo buildings: {non_extrativo_count}")
for tp, cnt in by_type.most_common():
    print(f"  {tp:30s}: {cnt}")
