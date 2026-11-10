import proto_sim4

p = next(p for p in proto_sim4.provs if p['nome'] == 'Nova Iorque')
print("Total levels in NY:", sum(proto_sim4.prov_levels[p['id']].values()))
for t, lvl in sorted(proto_sim4.prov_levels[p['id']].items()):
    print(f"  {t:30}: {lvl}")
