import os
import subprocess
import re

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
for line in proc.stdout.split("\n"):
    for pid in ["usa_3514", "usa_3536", "usa_3529", "usa_3559", "usa_3550", "gum_00"]:
        if f'id:"{pid}"' in line:
            m_nom = re.search(r'nome:"([^"]+)"', line)
            m_lim = re.search(r'limites_recursos:\{(.*?)\}', line)
            print(pid, m_nom.group(1) if m_nom else "", m_lim.group(1) if m_lim else "{}")
