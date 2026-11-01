#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Script de validação para os dados em formato RON da pasta enciclopedia:
- tutorial.ron
- conceitos.ron
- tooltips.ron
- mensagens.ron

Verifica:
1. Sintaxe balanceada e parsing do RON
2. Conformidade de IDs com snake_case ASCII
3. Unicidade absoluta de chaves/IDs (sem duplicatas)
4. Integridade referencial cruzada (conceitos_relacionados)
5. Sequência contínua de ordem do tutorial (1..25)
6. Correspondência exata dos placeholders nos templates de mensagens
"""

import sys
import os
import re

PASTA_ENCICLOPEDIA = os.path.dirname(os.path.abspath(__file__))

class SimpleRonTokenizer:
    def __init__(self, text):
        # Remove comentários de linha (//) e de bloco (/* */)
        text = re.sub(r'//.*', '', text)
        text = re.sub(r'/\*.*?\*/', '', text, flags=re.DOTALL)
        self.text = text
        self.pos = 0
        self.length = len(text)

    def skip_whitespace_and_commas(self):
        while self.pos < self.length and (self.text[self.pos].isspace() or self.text[self.pos] == ','):
            self.pos += 1

    def parse_string(self):
        assert self.text[self.pos] == '"'
        self.pos += 1
        chars = []
        while self.pos < self.length:
            c = self.text[self.pos]
            if c == '\\':
                self.pos += 1
                if self.pos < self.length:
                    escaped = self.text[self.pos]
                    if escaped == 'n': chars.append('\n')
                    elif escaped == 't': chars.append('\t')
                    elif escaped == 'r': chars.append('\r')
                    elif escaped == '"': chars.append('"')
                    elif escaped == '\\': chars.append('\\')
                    else: chars.append(escaped)
                self.pos += 1
            elif c == '"':
                self.pos += 1
                return ''.join(chars)
            else:
                chars.append(c)
                self.pos += 1
        raise ValueError("String não terminada")

    def parse_value(self):
        self.skip_whitespace_and_commas()
        if self.pos >= self.length:
            return None

        c = self.text[self.pos]
        if c == '"':
            return self.parse_string()
        if c == '[':
            return self.parse_list()
        if c == '(':
            return self.parse_struct_body()

        # Identificador, número, Some(...), None
        start = self.pos
        while self.pos < self.length and (self.text[self.pos].isalnum() or self.text[self.pos] in '_-.'):
            self.pos += 1
        ident = self.text[start:self.pos]
        self.skip_whitespace_and_commas()

        if self.pos < self.length and self.text[self.pos] == '(':
            # É uma struct nomeada ou enum com payload como Some(...)
            self.pos += 1
            if ident == 'Some':
                val = self.parse_value()
                self.skip_whitespace_and_commas()
                assert self.text[self.pos] == ')', f"Esperado ')' em Some, encontrado {self.text[self.pos]}"
                self.pos += 1
                return {"_type": "Some", "value": val}
            else:
                # Struct nomeada, ex: PassoTutorial(...)
                fields = self.parse_struct_fields()
                fields["_struct"] = ident
                return fields

        if ident == 'None':
            return None
        if ident == 'true':
            return True
        if ident == 'false':
            return False

        # Tenta converter para número
        try:
            if '.' in ident:
                return float(ident)
            return int(ident)
        except ValueError:
            return ident

    def parse_list(self):
        assert self.text[self.pos] == '['
        self.pos += 1
        items = []
        while True:
            self.skip_whitespace_and_commas()
            if self.pos >= self.length:
                raise ValueError("Lista não fechada com ']'")
            if self.text[self.pos] == ']':
                self.pos += 1
                return items
            val = self.parse_value()
            if val is not None or (self.pos < self.length and self.text[self.pos] != ']'):
                items.append(val)

    def parse_struct_fields(self):
        fields = {}
        while True:
            self.skip_whitespace_and_commas()
            if self.pos >= self.length:
                raise ValueError("Struct não fechada com ')'")
            if self.text[self.pos] == ')':
                self.pos += 1
                return fields

            # Lê nome do campo
            start = self.pos
            while self.pos < self.length and (self.text[self.pos].isalnum() or self.text[self.pos] == '_'):
                self.pos += 1
            key = self.text[start:self.pos]
            self.skip_whitespace_and_commas()
            if self.pos >= self.length or self.text[self.pos] != ':':
                raise ValueError(f"Esperado ':' após campo '{key}', posição {self.pos}")
            self.pos += 1  # pula ':'
            val = self.parse_value()
            fields[key] = val


def carregar_ron(caminho_arquivo):
    if not os.path.exists(caminho_arquivo):
        raise FileNotFoundError(f"Arquivo não encontrado: {caminho_arquivo}")
    with open(caminho_arquivo, 'r', encoding='utf-8') as f:
        conteudo = f.read()
    parser = SimpleRonTokenizer(conteudo)
    return parser.parse_value()


def main():
    print("=" * 70)
    print("VALIDADOR DE DADOS DA ENCICLOPÉDIA - GUERRA FRIA: 1945")
    print("=" * 70)

    erros = []
    avisos = []
    todos_os_ids = set()

    # 1. Validar tutorial.ron
    caminho_tutorial = os.path.join(PASTA_ENCICLOPEDIA, "tutorial.ron")
    print(f"\n[1/4] Validando: tutorial.ron...")
    try:
        passos = carregar_ron(caminho_tutorial)
        if not isinstance(passos, list):
            erros.append("tutorial.ron deve conter uma lista []")
            passos = []
        print(f"  -> {len(passos)} passos encontrados.")

        ids_tutorial = set()
        ordens = []
        for i, passo in enumerate(passos, start=1):
            pid = passo.get("id")
            ordem = passo.get("ordem")
            titulo = passo.get("titulo")
            texto = passo.get("texto")
            hud = passo.get("elemento_hud")

            if not pid or not re.match(r'^[a-z0-9_]+$', pid):
                erros.append(f"Passo #{i} possui id inválido ou não snake_case: '{pid}'")
            if pid in todos_os_ids:
                erros.append(f"ID duplicado detectado: '{pid}' em tutorial.ron")
            todos_os_ids.add(pid)
            ids_tutorial.add(pid)

            if ordem is not None:
                ordens.append(ordem)
            else:
                erros.append(f"Passo '{pid}' não tem campo 'ordem'")

            if not titulo or len(titulo) < 5:
                erros.append(f"Passo '{pid}' tem título ausente ou curto demais")
            if not texto or len(texto) < 30:
                erros.append(f"Passo '{pid}' tem texto explicativo insuficiente")
            if not hud:
                erros.append(f"Passo '{pid}' não possui 'elemento_hud'")

        if ordens != list(range(1, len(passos) + 1)):
            erros.append(f"A sequência de 'ordem' no tutorial está irregular: {ordens}")

    except Exception as e:
        erros.append(f"Falha ao ler tutorial.ron: {e}")

    # 2. Validar conceitos.ron
    caminho_conceitos = os.path.join(PASTA_ENCICLOPEDIA, "conceitos.ron")
    print(f"\n[2/4] Validando: conceitos.ron...")
    mapa_conceitos = {}
    try:
        conceitos = carregar_ron(caminho_conceitos)
        if not isinstance(conceitos, list):
            erros.append("conceitos.ron deve conter uma lista []")
            conceitos = []
        print(f"  -> {len(conceitos)} conceitos encontrados.")

        ids_conceitos = set()
        for i, c in enumerate(conceitos, start=1):
            cid = c.get("id")
            nome = c.get("nome")
            categoria = c.get("categoria")
            mecanica = c.get("definicao_mecanica")
            historico = c.get("contexto_historico")
            relacionados = c.get("conceitos_relacionados", [])

            if not cid or not re.match(r'^[a-z0-9_]+$', cid):
                erros.append(f"Conceito #{i} possui id inválido: '{cid}'")
            if cid in todos_os_ids:
                erros.append(f"ID duplicado detectado: '{cid}' em conceitos.ron")
            todos_os_ids.add(cid)
            ids_conceitos.add(cid)
            mapa_conceitos[cid] = c

            if not nome or len(nome) < 2:
                erros.append(f"Conceito '{cid}' tem nome inválido")
            if not categoria or not re.match(r'^[a-z0-9_]+$', categoria):
                erros.append(f"Conceito '{cid}' tem categoria inválida: '{categoria}'")
            if not mecanica or len(mecanica) < 20:
                erros.append(f"Conceito '{cid}' tem 'definicao_mecanica' insuficiente")
            if not historico or len(historico) < 20:
                erros.append(f"Conceito '{cid}' tem 'contexto_historico' insuficiente")
            if not isinstance(relacionados, list):
                erros.append(f"Conceito '{cid}' deve ter 'conceitos_relacionados' como lista")

        # Validar referências cruzadas entre conceitos
        for cid, c in mapa_conceitos.items():
            for ref in c.get("conceitos_relacionados", []):
                if ref not in ids_conceitos:
                    erros.append(f"Conceito '{cid}' referencia conceito inexistente '{ref}'")

    except Exception as e:
        erros.append(f"Falha ao ler conceitos.ron: {e}")

    # 3. Validar tooltips.ron
    caminho_tooltips = os.path.join(PASTA_ENCICLOPEDIA, "tooltips.ron")
    print(f"\n[3/4] Validando: tooltips.ron...")
    try:
        tooltips = carregar_ron(caminho_tooltips)
        if not isinstance(tooltips, list):
            erros.append("tooltips.ron deve conter uma lista []")
            tooltips = []
        print(f"  -> {len(tooltips)} tooltips encontradas.")

        ids_tooltips = set()
        for i, t in enumerate(tooltips, start=1):
            tid = t.get("id")
            alvo = t.get("alvo_ui")
            titulo = t.get("titulo")
            desc = t.get("descricao")

            if not tid or not re.match(r'^[a-z0-9_]+$', tid):
                erros.append(f"Tooltip #{i} possui id inválido: '{tid}'")
            if tid in todos_os_ids:
                erros.append(f"ID duplicado detectado: '{tid}' em tooltips.ron")
            todos_os_ids.add(tid)
            ids_tooltips.add(tid)

            if not alvo:
                erros.append(f"Tooltip '{tid}' não possui 'alvo_ui'")
            if not titulo:
                erros.append(f"Tooltip '{tid}' não possui 'titulo'")
            if not desc:
                erros.append(f"Tooltip '{tid}' não possui 'descricao'")

    except Exception as e:
        erros.append(f"Falha ao ler tooltips.ron: {e}")

    # 4. Validar mensagens.ron
    caminho_mensagens = os.path.join(PASTA_ENCICLOPEDIA, "mensagens.ron")
    print(f"\n[4/4] Validando: mensagens.ron...")
    try:
        mensagens = carregar_ron(caminho_mensagens)
        if not isinstance(mensagens, list):
            erros.append("mensagens.ron deve conter uma lista []")
            mensagens = []
        print(f"  -> {len(mensagens)} modelos de mensagem encontrados.")

        ids_mensagens = set()
        for i, m in enumerate(mensagens, start=1):
            mid = m.get("id")
            cat = m.get("categoria")
            alerta = m.get("tipo_alerta")
            tit_tmpl = m.get("titulo_template")
            msg_tmpl = m.get("mensagem_template")
            placeholders = m.get("placeholders", [])

            if not mid or not re.match(r'^[a-z0-9_]+$', mid):
                erros.append(f"Mensagem #{i} possui id inválido: '{mid}'")
            if mid in todos_os_ids:
                erros.append(f"ID duplicado detectado: '{mid}' em mensagens.ron")
            todos_os_ids.add(mid)
            ids_mensagens.add(mid)

            if alerta not in ("critico", "importante", "oportunidade", "informativo"):
                erros.append(f"Mensagem '{mid}' possui tipo_alerta desconhecido: '{alerta}'")

            # Extrai placeholders com regex de dentro dos templates
            encontrados = set(re.findall(r'\{([a-zA-Z0-9_]+)\}', (tit_tmpl or "") + " " + (msg_tmpl or "")))
            declarados = set(placeholders)

            # Verifica se todos os placeholders declarados aparecem no template
            nao_usados = declarados - encontrados
            if nao_usados:
                avisos.append(f"Mensagem '{mid}' declara placeholders não utilizados no texto: {nao_usados}")

            # Verifica se há placeholders no template não declarados no vetor
            nao_declarados = encontrados - declarados
            if nao_declarados:
                erros.append(f"Mensagem '{mid}' usa placeholders não declarados na lista: {nao_declarados}")

    except Exception as e:
        erros.append(f"Falha ao ler mensagens.ron: {e}")

    # Relatório Final
    print("\n" + "=" * 70)
    print("RELATÓRIO DE RESULTADOS DA VALIDAÇÃO")
    print("=" * 70)
    print(f"Total de Chaves / IDs Validados: {len(todos_os_ids)}")
    print(f"Total de Passos de Tutorial:    {len(passos)}")
    print(f"Total de Verbetes da Enciclopédia: {len(mapa_conceitos)}")
    print(f"Total de Tooltips de Interface: {len(tooltips)}")
    print(f"Total de Modelos de Mensagens:  {len(mensagens)}")

    if avisos:
        print(f"\nAvisos ({len(avisos)}):")
        for av in avisos:
            print(f"  [AVISO] {av}")

    if erros:
        print(f"\nERROS ENCONTRADOS ({len(erros)}):")
        for err in erros:
            print(f"  [ERRO] {err}")
        print("\nSTATUS: VALIDAÇÃO FALHOU!")
        sys.exit(1)
    else:
        print("\nSTATUS: TODAS AS VALIDAÇÕES PASSARAM COM SUCESSO! ZERO ERROS!")
        sys.exit(0)


if __name__ == '__main__':
    main()
