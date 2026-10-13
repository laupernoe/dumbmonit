"""One language per build, chosen by READTHEDOCS_LANGUAGE.

Read the Docs builds one project per language (/en/latest/, /fr/latest/, ...) and exposes
the language of the project being built in READTHEDOCS_LANGUAGE (lower case, e.g. "pt-br",
"zh-cn"). Locally the variable is absent and the build is English.

The pages of the chosen language (page.<suffix>.md) replace the English ones at the root of
the build; a page without translation stays in English with a notice.
"""

import os
import re
from pathlib import Path

import yaml
from mkdocs.localization import parse_locale
from mkdocs.structure.files import File

# RTD code -> (suffix of the translated files, Material theme.language,
# label in the switcher, notice for a page that is not translated yet)
LANGUAGES = {
    "en": (None, "en", "English", None),
    "fr": ("fr", "fr", "Français",
           "Cette page n'est pas encore traduite : voici la version anglaise."),
    "de": ("de", "de", "Deutsch",
           "Diese Seite ist noch nicht übersetzt: hier die englische Fassung."),
    "es": ("es", "es", "Español",
           "Esta página aún no está traducida: se muestra la versión en inglés."),
    "it": ("it", "it", "Italiano",
           "Questa pagina non è ancora tradotta: ecco la versione inglese."),
    "pt": ("pt", "pt", "Português",
           "Esta página ainda não foi traduzida: apresenta-se a versão em inglês."),
    "pt-br": ("pt-BR", "pt-BR", "Português (Brasil)",
              "Esta página ainda não foi traduzida: aqui está a versão em inglês."),
    "ru": ("ru", "ru", "Русский",
           "Эта страница ещё не переведена: показана английская версия."),
    "zh-cn": ("zh-Hans", "zh", "简体中文", "此页面尚未翻译，以下为英文版本。"),
}
RTD_HOST = "https://dumbmonit.readthedocs.io"
SUFFIXES = {spec[0] for spec in LANGUAGES.values() if spec[0]}
RE_SUFFIXED = re.compile(r"^(.*)\.(%s)(\.[^./]+)$" % "|".join(map(re.escape, SUFFIXES)))

_state = {"code": "en", "untranslated": set()}


def _code() -> str:
    code = (os.environ.get("READTHEDOCS_LANGUAGE") or "en").strip().lower().replace("_", "-")
    if code not in LANGUAGES:
        raise SystemExit(
            f"READTHEDOCS_LANGUAGE={code!r} is not handled by hooks/rtd_language.py "
            f"(known: {', '.join(LANGUAGES)})"
        )
    return code


def _translate_nav(items, titles):
    out = []
    for item in items:
        if isinstance(item, dict):
            ((title, value),) = item.items()
            if isinstance(value, list):
                value = _translate_nav(value, titles)
            out.append({titles.get(title, title): value})
        else:
            out.append(item)
    return out


def on_config(config):
    code = _state["code"] = _code()
    suffix, material, _, _ = LANGUAGES[code]

    canonical = os.environ.get("READTHEDOCS_CANONICAL_URL")
    if canonical:
        config.site_url = canonical if canonical.endswith("/") else canonical + "/"

    config.theme["language"] = material
    config.theme["locale"] = parse_locale(material.replace("-", "_"))
    for name, plugin in config.plugins.items():
        if name.endswith("search"):
            # Material's search plugin took its language, separator and stemming pipeline
            # from the theme language (English) before this hook ran: derive them again.
            plugin.config.lang = []
            plugin.config.separator = ""
            plugin.config.pipeline = None
            plugin.on_config(config)

    config.extra["alternate"] = [
        {"name": spec[2], "link": f"{RTD_HOST}/{rtd}/latest/", "lang": spec[1]}
        for rtd, spec in LANGUAGES.items()
    ]

    if suffix:
        data = yaml.safe_load((Path(__file__).parent / "nav_translations.yml").read_text())
        config.nav = _translate_nav(config.nav, data.get(suffix, {}))
    return config


def on_files(files, config):
    suffix = LANGUAGES[_state["code"]][0]
    translated = {}
    for f in list(files):
        m = RE_SUFFIXED.match(f.src_uri)
        if not m:
            continue
        files.remove(f)
        if m.group(2) == suffix:
            translated[m.group(1) + m.group(3)] = f
    for base, tr in translated.items():
        old = files.get_file_from_path(base)
        if old:
            files.remove(old)
        new = File(base, config.docs_dir, config.site_dir, config.use_directory_urls)
        new.abs_src_path = tr.abs_src_path
        files.append(new)
    if suffix:
        _state["untranslated"] = {
            f.src_uri for f in files.documentation_pages() if f.src_uri not in translated
        }
    return files


def on_page_markdown(markdown, page, config, files):
    notice = LANGUAGES[_state["code"]][3]
    if notice and page.file.src_uri in _state["untranslated"]:
        page.meta["fallback_notice"] = notice
    return markdown
