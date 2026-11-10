import os
import subprocess
import re
from collections import Counter

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

provs_with_recursos = 0
total_provs = 0
by_country = Counter()

for line in j_text.split("\n"):
    line = line.strip()
    if not line.startswith("(") or not line.endswith("),"):
        continue
    total_provs += 1
    m_pais = re.search(r'pais_id:"([^"]+)"', line)
    m_lim = re.search(r'limites_recursos:\{(.*?)\}', line)
    if m_lim and len(m_lim.group(1).strip()) > 0:
        provs_with_recursos += 1
        if m_pais:
            by_country[m_pais.group(1)] += 1

print(f"Total provs: {total_provs}, with limites_recursos: {provs_with_recursos}")
print("Top countries with limites_recursos:", by_country.most_common(10))
