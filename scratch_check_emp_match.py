import os
import subprocess
import re
from collections import defaultdict

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

# Check Iowa in jogavel
for line in j_text.split("\n"):
    if 'id:"usa_3529"' in line:
        # Sum pop empregados by type
        m_pops = re.search(r'pops:\[(.*?)\]', line)
        pop_emp = defaultdict(float)
        pop_tot = defaultdict(float)
        if m_pops:
            for pop_m in re.finditer(r'tipo_pop_id:"([^"]+)",.*?populacao:([0-9.]+),empregados:([0-9.]+)', m_pops.group(1)):
                tp = pop_m.group(1)
                pop_tot[tp] += float(pop_m.group(2))
                pop_emp[tp] += float(pop_m.group(3))
        
        # Sum predio empregados_por_tipo
        b_emp = defaultdict(float)
        m_pred = re.search(r'predios:\[(.*)\]', line)
        if m_pred:
            for b in re.finditer(r'tipo_predio_id:"([^"]+)",nivel:(\d+).*?empregados_por_tipo:\{(.*?)\}', m_pred.group(1)):
                tp_b = b.group(1)
                emp_str = b.group(3)
                for e_m in re.finditer(r'"([^"]+)":([0-9.]+)', emp_str):
                    b_emp[e_m.group(1)] += float(e_m.group(2))
        
        print("=== Iowa in jogavel ===")
        for tp in sorted(pop_tot.keys()):
            print(f"  {tp:15s}: pop={pop_tot[tp]:8.1f}, emp={pop_emp[tp]:8.1f}, b_emp={b_emp[tp]:8.1f}")
