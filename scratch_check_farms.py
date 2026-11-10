import os
import subprocess
import re

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

total_terra = 0
total_fazendas = 0
total_infra = 0
for line in j_text.split("\n"):
    if 'pais_id:"usa"' not in line:
        continue
    m_terra = re.search(r'terra_aravel_total:(\d+)', line)
    if m_terra:
        total_terra += int(m_terra.group(1))
    m_infra = re.search(r'infraestrutura_base:([0-9.]+)', line)
    if m_infra:
        total_infra += float(m_infra.group(1))
    for m in re.finditer(r'tipo_predio_id:"fazenda_[^"]+",nivel:(\d+)', line):
        total_fazendas += int(m.group(1))

print(f"USA in jogavel: total_terra={total_terra}, total_fazendas={total_fazendas}, ratio={total_fazendas/total_terra:.4f}")
print(f"USA total infra base={total_infra}")
