import subprocess
import re

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

print("JOGAVEL porto_pesca in USA:")
for line in proc.stdout.splitlines():
    if 'pais_id:"usa"' in line:
        m_p = re.search(r'tipo_predio_id:"porto_pesca"[^)]*?nivel:([0-9]+)', line)
        if m_p:
            m_nome = re.search(r'nome:"([^"]+)"', line)
            print(f"  {m_nome.group(1):25}: nivel {m_p.group(1)}")
