# GPU akcelerovaná aplikace na malování v Rustu

![Licence - MIT nebo Apache-2.0](https://img.shields.io/badge/Licence-MIT_nebo_Apache--2.0-blue)

[EN](README.md)/[CZ](README-CZ.md)

## Použití

Viz také [sekci zajímavé funkce](#zajímavé-funkce).

### Kreslení tvarů

1. **Výběr tvarů:** Na horní liště vyberte požadovaný tvar.
2. **Umístění kontrolních bodů:** Klikněte na plátno pro umístění kontrolních bodů.
3. **Aktivace režimu úprav:** Automaticky pro ne-polygony po umístění 2 bodů. U polygonů stiskněte ENTER po umístění bodů.
4. **Úprava tvaru:** Přetáhněte kontrolní body pro úpravu tvaru.
5. **Dokončení:** Stiskněte ENTER pro vykreslení tvaru.
6. **Zrušení:** Během libovolného kroku stiskněte ESCAPE pro smazání aktuálního tvaru.

### Další akce

1. **Výběr akce:** Na horní liště vyberte mazání obsahu nebo vyplnění tvarů/oblastí.
2. **Provedení akce:** Mažete tažením kurzoru, vyplňujete kliknutím.

## Zajímavé funkce

- Kreslení čar, obdélníků, kružnic nebo polygonů, mazání obsahu a vyplňování tvarů/oblastí barvou.
- Výběr barvy z předvoleb nebo pomocí palety barvy s podporou průhlednosti.
- Přizpůsobení tloušťky čar a obrysů pro všechny nástroje.
- Vyhlazování okrajů (anti-aliasing) s volbou stupně.
- Podpora přerušovaných čar s nastavitelnou délkou čárky a mezery.
- Přiblížení/oddálení s konfigurovatelnou rychlostí (kolečkem myši nebo gesty touchpadu).
- Posouvání po plátně pomocí myši nebo touchpadu.
- Vypnutí náhledu vykreslování během kreslení pro lepší výkon na starším hardwaru.
- Nastavení citlivosti zachycení bodů.
- Přizpůsobení nastavení uživatelského rozhraní dle preferencí.

## Sestavení

### Pomocí Cargo

```sh
cargo b --release
./target/release/paint
```

### Pomocí Nix

```sh
nix build
./result/bin/paint
```

#### Dev-shell

```sh
nix develop
```

## Implementační detaily

- **Programovací jazyk:** Aplikace je napsána v [Rustu](https://www.rust-lang.org/).
- **GPU akcelerace:** Pro všechny GPU operace je využívá knihovna [WGPU](https://wgpu.rs/).
- **Správa oken:** Pro vytváření a správu oken je používá knihovna [winit](https://github.com/rust-windowing/winit).
- **Uživatelské rozhraní:** Implementováno pomocí knihovny [egui](https://github.com/emilk/egui).
- **Způsob vykreslování:** Tvary se vykreslují na texturu pomocí [signed distance funkcí (SDFs)](https://iquilezles.org/articles/distfunctions2d/) v compute shaderu.
- **Shader jazyk:** Všechny shadery (včetně výpočetního shaderu) jsou napsány ve [WGSL](https://www.w3.org/TR/WGSL/).

## Licence

- Tento projekt je distribuován pod podmínkami buď [MIT License](LICENSE-MIT) nebo [Apache License Version 2.0](LICENSE-APACHE) dle vašeho výběru.
- Projekt obsahuje [ikony](src/icons) z [Krita projektu](https://github.com/KDE/krita) licencované pod [Creative Commons Attribution-ShareAlike 4.0 International License (CC BY-SA 4.0)](https://creativecommons.org/licenses/by-sa/4.0/).
