import os
import subprocess
import re

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

# Check which pop types have subsistencia: true in catalogo
with open("assets/data/economia/tipos_pop.ron", "r", encoding="utf-8") as f:
    cat = f.read()

subsistencia_types = set()
for m in re.finditer(r'\(id:"([^"]+)",(?:.*?subsistencia:\s*(true|false))?', cat):
    if m.group(2) == 'true':
        subsistencia_types.add(m.group(1))

print("Subsistencia types:", subsistencia_types)

def check_usa_desemp(content, label):
    lines = content.strip().split("\n")
    pop_trab = 0.0
    desemp = 0.0
    for line in lines:
        if 'pais_id:"usa"' not in line:
            continue
        m_pops = re.search(r'pops:\[(.*?)\]', line)
        if m_pops:
            for pop_m in re.finditer(r'tipo_pop_id:"([^"]+)",.*?populacao:([0-9.]+),empregados:([0-9.]+)', m_pops.group(1)):
                tp = pop_m.group(1)
                pop = float(pop_m.group(2))
                emp = float(pop_m.group(3))
                if tp not in subsistencia_types:
                    pop_trab += pop
                    desemp += max(0.0, pop - emp)
    rate = desemp / pop_trab if pop_trab > 0 else 0
    print(f"{label} USA desemprego: {rate*100:.2f}% (pop_trab={pop_trab/1e6:.1f}M desemp={desemp/1e6:.1f}M)")

proc_j = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
check_usa_desemp(proc_j.stdout, "JOGAVEL")

proc_1b = subprocess.run(["git", "show", "1bdd42ac:assets/data/economia/provincias.ron"], capture_output=True, text=True)
check_usa_desemp(proc_1b.stdout, "1BDD42AC")
