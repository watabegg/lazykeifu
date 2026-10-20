# Translation catalogs

`en.json` is extracted from literal `tr!` and `trf!` calls. Each other JSON file
maps the same English source keys to translated messages. Catalogs are embedded
in the binary. English is the fallback for missing or empty messages.

After editing UI text, run `python3 scripts/i18n-catalog.py --extract`, translate
the new keys in every catalog, then run `python3 scripts/i18n-catalog.py`.
Preserve named placeholders, leading/trailing whitespace, keybindings, and Git
command/option names. Pass repository data as placeholder arguments, never as
translation keys. Arguments are interpolated once, without translating their text.

Common terminology and matching translations are based on
[Lazygit](https://github.com/jesseduffield/lazygit/tree/4e1eebe278790f257a875da6633234391a5a85b3/pkg/i18n),
under the retained [MIT license](LICENSE.lazygit). Other translations cover
pedigit's own UI and operation messages. The language set matches that upstream
revision; it is not fetched at runtime.
