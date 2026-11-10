import subprocess
import re

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

jog_vagas_by_type = {}
for line in proc.stdout.splitlines():
    if 'pais_id:"usa"' in line:
        for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)[^)]*?empregados_por_tipo:\{([^}]+)\}', line):
            tp = m.group(1)
            lvl = int(m.group(2))
            vagas = sum(float(x.group(1)) for x in re.finditer(r':([0-9.]+)', m.group(3)))
            if tp not in jog_vagas_by_type and lvl > 0:
                jog_vagas_by_type[tp] = (lvl, vagas, vagas / lvl)

cur_vagas_by_type = {}
with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    for line in f:
        if 'pais_id:"usa"' in line:
            for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)[^)]*?empregados_por_tipo:\{([^}]+)\}', line):
                tp = m.group(1)
                lvl = int(m.group(2))
                vagas = sum(float(x.group(1)) for x in re.finditer(r':([0-9.]+)', m.group(3)))
                if tp not in cur_vagas_by_type and lvl > 0:
                    cur_vagas_by_type[tp] = (lvl, vagas, vagas / lvl)

for tp in sorted(jog_vagas_by_type.keys()):
    j_lvl, j_v, j_ratio = jog_vagas_by_type[tp]
    c_lvl, c_v, c_ratio = cur_vagas_by_type[tp]
    print(f"{tp:30} jog_ratio={j_ratio:.1f} cur_ratio={c_ratio:.1f}")
