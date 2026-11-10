import proto_sim5

total_vagas_usa = sum(
    sum(proto_sim5.prov_levels[p['id']][t] * proto_sim5.vagas_ratio.get(t, 450.0) for t in proto_sim5.prov_levels[p['id']])
    for p in proto_sim5.provs
)
print("Actual total vagas in USA:", total_vagas_usa)
