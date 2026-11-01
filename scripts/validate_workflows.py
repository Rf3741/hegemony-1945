#!/usr/bin/env python3
"""
Script de validação de sintaxe e regras de negócio dos Workflows do GitHub Actions.
Utiliza PyYAML instalado em pydeps.
"""

import sys
import os

# Adiciona dependências locais sem venv conforme regras do projeto
PYDEPS_PATH = os.environ.get("PYDEPS", "/home/rafael/claude/guerra-fria-staging/build_ci/pydeps")
if PYDEPS_PATH not in sys.path:
    sys.path.insert(0, PYDEPS_PATH)

try:
    import yaml
except ImportError as err:
    print(f"ERRO: Não foi possível importar pyyaml de {PYDEPS_PATH}: {err}")
    sys.exit(1)


def validar_arquivo_yaml(caminho_arquivo):
    """Lê e analisa a sintaxe do arquivo YAML."""
    if not os.path.exists(caminho_arquivo):
        raise FileNotFoundError(f"Arquivo não encontrado: {caminho_arquivo}")

    with open(caminho_arquivo, "r", encoding="utf-8") as f:
        conteudo = f.read()

    try:
        dados = yaml.safe_load(conteudo)
    except yaml.YAMLError as exc:
        raise ValueError(f"Erro de sintaxe YAML em {caminho_arquivo}: {exc}")

    return dados, conteudo


def validar_ci_yml(caminho):
    print(f">> Validando CI Workflow: {caminho}")
    dados, texto = validar_arquivo_yaml(caminho)

    assert "name" in dados, "ci.yml deve ter um campo 'name'"
    assert "on" in dados or True in dados, "ci.yml deve definir gatilhos 'on'"
    assert "jobs" in dados, "ci.yml deve conter 'jobs'"

    jobs = dados["jobs"]
    print(f"   Jobs detectados: {list(jobs.keys())}")

    # Validações solicitadas na tarefa: fmt, clippy, test, build
    assert "fmt" in jobs, "ci.yml deve conter o job 'fmt'"
    assert "clippy" in jobs, "ci.yml deve conter o job 'clippy'"
    assert "test" in jobs, "ci.yml deve conter o job 'test'"
    assert "build" in jobs, "ci.yml deve conter o job 'build'"

    # Verifica runners ubuntu-latest e windows-latest
    test_runs = str(jobs["test"])
    build_runs = str(jobs["build"])
    assert "ubuntu-latest" in test_runs and "windows-latest" in test_runs, (
        "Job 'test' deve rodar em ubuntu-latest e windows-latest"
    )
    assert "ubuntu-latest" in build_runs and "windows-latest" in build_runs, (
        "Job 'build' deve rodar em ubuntu-latest e windows-latest"
    )

    # Verifica cache do cargo
    assert "rust-cache" in texto or "actions/cache" in texto, (
        "ci.yml deve utilizar cache do cargo (ex: Swatinem/rust-cache)"
    )

    # Verifica dependências do Bevy no Linux
    deps_bevy = [
        "libasound2-dev",
        "libudev-dev",
        "libwayland-dev",
        "libxkbcommon-dev",
    ]
    for dep in deps_bevy:
        assert dep in texto, f"ci.yml deve instalar a dependência '{dep}' para Bevy no Linux"

    print("   [OK] Todos os requisitos de ci.yml foram validados com sucesso!")


def validar_release_yml(caminho):
    print(f">> Validando Release Workflow: {caminho}")
    dados, texto = validar_arquivo_yaml(caminho)

    assert "name" in dados, "release.yml deve ter um campo 'name'"
    assert "jobs" in dados, "release.yml deve conter 'jobs'"

    # Gatilho de tags v*
    assert "v*" in texto, "release.yml deve ser acionado em tags v*"

    # Validação de artefatos Windows (.zip), Linux (.tar.gz) e macOS (.tar.gz)
    assert ".tar.gz" in texto, "release.yml deve gerar arquivo .tar.gz para Linux e macOS"
    assert ".zip" in texto, "release.yml deve gerar arquivo .zip para Windows"
    assert "macos-latest" in texto, "release.yml deve incluir o runner macos-latest"
    assert "aarch64-apple-darwin" in texto, "release.yml deve incluir target aarch64-apple-darwin"
    assert "x86_64-apple-darwin" in texto, "release.yml deve incluir target x86_64-apple-darwin"
    assert "runner.os == 'macOS'" in texto, "release.yml deve incluir etapa de empacotamento para macOS"

    # Validação de inclusão da pasta assets
    assert "assets" in texto, "release.yml deve incluir a pasta 'assets' nos pacotes"

    # Validação da versão no nome do arquivo
    assert "github.ref_name" in texto, "release.yml deve incluir a versão/tag no nome do arquivo"

    # Validação de publicação de GitHub Release
    assert "action-gh-release" in texto or "gh release create" in texto, (
        "release.yml deve publicar uma GitHub Release"
    )

    print("   [OK] Todos os requisitos de release.yml foram validados com sucesso!")


def main():
    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    ci_path = os.path.join(base_dir, ".github/workflows/ci.yml")
    release_path = os.path.join(base_dir, ".github/workflows/release.yml")

    print("==================================================================")
    print(" Validação de Sintaxe e Estrutura YAML dos Workflows")
    print("==================================================================")

    try:
        validar_ci_yml(ci_path)
        validar_release_yml(release_path)
        print("==================================================================")
        print(" SUCESSO: Todos os arquivos YAML são válidos e cumprem os requisitos!")
        print("==================================================================")
        sys.exit(0)
    except Exception as exc:
        print(f"FALHA na validação: {exc}")
        sys.exit(1)


if __name__ == "__main__":
    main()
