import os
import subprocess
import re
from collections import defaultdict

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

def analyze_file(content, label):
    print(f"\n=== Analyzing {label} ===")
    lines = content.strip().split("\n")
    prov_count = 0
    total_buildings = 0
    total_levels = 0
    by_country_predios = defaultdict(int)
    by_country_niveis = defaultdict(int)
    by_country_tipo_niveis = defaultdict(lambda: defaultdict(int))
    by_country_pops = defaultdict(float)
    by_country_emp = defaultdict(float)

    for line in lines:
        line = line.strip()
        if not line.startswith("(") or not line.endswith("),"):
            continue
        prov_count += 1
        m_pais = re.search(r'pais_id:"([^"]+)"', line)
        if not m_pais:
            continue
        pais = m_pais.group(1)

        # pops
        m_pops = re.search(r'pops:\[(.*?)\]', line)
        if m_pops:
            for pop_m in re.finditer(r'populacao:([0-9.]+),empregados:([0-9.]+)', m_pops.group(1)):
                pop = float(pop_m.group(1))
                emp = float(pop_m.group(2))
                by_country_pops[pais] += pop
                by_country_emp[pais] += emp

        # predios
        p_idx = line.find("predios:[")
        if p_idx != -1:
            p_content = line[p_idx + 8:]
            for b in re.finditer(r'tipo_predio_id:"([^"]+)",nivel:(\d+)', p_content):
                tipo = b.group(1)
                nivel = int(b.group(2))
                total_buildings += 1
                total_levels += nivel
                by_country_predios[pais] += 1
                by_country_niveis[pais] += nivel
                by_country_tipo_niveis[pais][tipo] += nivel

    print(f"Provinces: {prov_count}, Buildings: {total_buildings}, Levels: {total_levels}")
    for c in ["usa", "sov", "bra", "ger", "gbr"]:
        pop_m = by_country_pops[c] / 1e6
        emp_rate = (by_country_emp[c] / by_country_pops[c] * 100) if by_country_pops[c] > 0 else 0
        print(f"  {c}: pop={pop_m:.1f}M emp={emp_rate:.1f}% (desemp={100-emp_rate:.1f}%) predios={by_country_predios[c]} niveis={by_country_niveis[c]}")
    return by_country_niveis, by_country_tipo_niveis

proc_j = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
analyze_file(proc_j.stdout, "JOGAVEL")

proc_1b = subprocess.run(["git", "show", "1bdd42ac:assets/data/economia/provincias.ron"], capture_output=True, text=True)
analyze_file(proc_1b.stdout, "1BDD42AC")

with open("assets/data/economia/provincias.ron", "r", encoding="utf-8") as f:
    analyze_file(f.read(), "CURRENT (HEAD/15aa9267)")
