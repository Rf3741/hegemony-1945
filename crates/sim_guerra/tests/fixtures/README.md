# Esquema de Dados Militares em RON (`tests/fixtures/`)

Este diretório contém os arquivos de dados de exemplo no formato **RON (Rusty Object Notation)** utilizados pelo motor de guerra puro `sim_guerra`.

---

## 1. Visão Geral dos Arquivos

| Arquivo | Descrição | Estrutura Rust Principal |
| :--- | :--- | :--- |
| `terrenos.ron` | Tipos de terreno, largura de combate, atrito e modificadores | `Vec<TipoTerreno>` |
| `unidades.ron` | Batalhões componentes (infantaria, blindados, artilharia, engenharia) | `Vec<TipoUnidade>` |
| `templates_divisao.ron` | Composição doutrinária das divisões por batalhões | `Vec<TemplateDivisao>` |
| `tecnologias_militares.ron` | Tecnologias militares pesquisadas e seus bônus operacionais | `Vec<TecnologiaMilitar>` |
| `tracos_comandante.ron` | Traços e especialidades táticas de generais e almirantes | `Vec<TracoComandante>` |
| `regioes.ron` | Províncias com terreno, portos, fortificações e pontos de vitória | `Vec<RegiaoMapa>` |
| `arestas.ron` | Grafo de adjacência entre regiões (rios, estreitos, distâncias em km) | `Vec<ArestaRegiao>` |
| `paises.ron` | Beligerantes soberanos, capitais e capacidade logística | `Vec<PaisMilitar>` |
| `comandantes.ron` | Oficiais generais, níveis de habilidade e traços atribuídos | `Vec<Comandante>` |
| `divisoes.ron` | Divisões ativas, efetivos, organização, moral e suprimento | `Vec<Divisao>` |
| `exercitos.ron` | Agrupamentos de divisões sob uma postura e comandante | `Vec<Exercito>` |
| `guerras.ron` | Conflitos armados ativos, beligerantes e pontuação de guerra | `Vec<Guerra>` |
| `config_guerra.ron` | Parâmetros de equilíbrio e critérios de capitulação/armistício | `ConfigGuerra` |

---

## 2. Identificadores e Convenções

- **Chave de Região (`adm1_code`)**: Código alfanumérico ASCII com hífen ou sublinhado, compativel com o *Natural Earth 10m Admin-1* (ex: `"USA-3521"`, `"SOV-1689"`, `"USA-0001"`).
- **Tag de País (`pais_tag`)**: Exatamente 3 letras maiúsculas ASCII (ex: `"USA"`, `"SOV"`, `"GER"`, `"GBR"`), em conformidade estrita com `nacoes/paises.ron`.
- **Identificadores Internos (`id`)**: `snake_case` ASCII rigoroso (`[a-z0-9_]`), sem caracteres acentuados ou maiúsculas.
- **Textos de Exibição (`nome_pt`)**: Português do Brasil (`pt-BR`).

---

## 3. Modelo Matemático dos Subsistemas

### 3.1. Suprimento Logístico e Fontes
O suprimento propaga-se a partir da **Capital** e de **Portos** controlados pelo beligerante através de território amigo.
A resistência de cada salto de região depende da infraestrutura efetiva e do terreno:

$$\text{CustoPasso} = \text{Terreno.CustoSuprimento} \times \frac{120.0}{20.0 + \text{InfraEfetiva}}$$

Divisões totalmente isoladas de qualquer capital ou porto amigo sofrem atrito diário e perda de organização.

### 3.2. Largura de Combate e Resolução Tática
Cada terreno dita a largura máxima da frente (`largura_combate`).
- **Defesa vs. Ruptura (*Breakthrough*)**: Ataques que superam a ruptura/defesa causam dano quadruplicado.
- **Blindagem vs. Perfuração**: Divisões com blindagem superior à perfuração inimiga sofrem $-40\%$ de dano de organização e causam $+25\%$ de dano.
- **Entrincheiramento**: Unidades paradas constroem entrincheiramento diário (acelerado por batalhões de engenharia), multiplicando a eficácia defensiva.

### 3.3. Frentes Automáticas e Avanço Região a Região
As fronteiras entre países beligerantes em guerra ativa são agrupadas automaticamente em componentes contíguos de combate (`Frente`).
Quando as divisões defensoras de uma região perdem a organização, recuam para território amigo adjacente (ou são destruídas se cercadas num bolsão), e as forças atacantes ocupam a região avançando a linha de frente.

### 3.4. Ocupação e Capitulação
- O país dono legítimo é registrado em `dono_tag`; o ocupante militar efetivo em `controlador_tag`.
- A perda da capital somada à perda de $\ge 80\%$ dos pontos de vitória (`pontos_vitoria`) dispara a capitulação incondicional.

### 3.5. Tratados de Paz
A API de tratados de paz suporta:
- **Paz Branca** (status quo ante bellum)
- **Anexação de Regiões** (transferência de soberania de jure)
- **Mudança de Regime** (imposição de ideologia / doutrina pró-vencedor)
- **Zonas de Ocupação Militar**
- **Armistício por Exaustão Mútua**
