import test_redist
for p in test_redist.provs:
    if 'Texas' in p['nome']:
        print(p['id'], p['nome'], p['recursos'], "oil level:", test_redist.prov_levels[p['id']]['poco_petroleo'])
