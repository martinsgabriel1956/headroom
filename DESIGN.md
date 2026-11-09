---
name: Headroom
description: The disk drawn as an architectural floor plan; every room's area is real recoverable space.
colors:
  paper: "#fbfbf9"
  paper-2: "#f0f0eb"
  ink: "#1a1a1a"
  graphite: "#555555"
  rule: "#d6d6d0"
  draft: "#2f5d8a"
  draft-tint: "#e6edf4"
  brick: "#8f2a1d"
  poche-line: "#cfcfc8"
  poche-swatch-line: "#8a8a84"
typography:
  wordmark:
    fontFamily: "Barlow, system-ui, sans-serif"
    fontSize: "19px"
    fontWeight: 700
    letterSpacing: "0.01em"
  headline:
    fontFamily: "Barlow, system-ui, sans-serif"
    fontSize: "20px"
    fontWeight: 700
  room-area-large:
    fontFamily: "Barlow, system-ui, sans-serif"
    fontSize: "24px"
    fontWeight: 600
    lineHeight: 1.15
    fontFeature: "tnum"
  room-area:
    fontFamily: "Barlow, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 600
    lineHeight: 1.15
    fontFeature: "tnum"
  room-name:
    fontFamily: "Barlow, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 700
    lineHeight: "18px"
    letterSpacing: "0.05em"
  body:
    fontFamily: "Barlow, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    fontFeature: "tnum"
  body-lead:
    fontFamily: "Barlow, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 400
  annotation:
    fontFamily: "Barlow, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
  label:
    fontFamily: "Barlow, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 600
    letterSpacing: "0.06em"
  stamp:
    fontFamily: "Barlow, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 700
    lineHeight: "16px"
    letterSpacing: "0.1em"
rounded:
  plate: "2px"
  control: "6px"
  dialog: "8px"
  key: "9999px"
spacing:
  gutter: "32px"
  topbar: "56px"
  footer: "84px"
  cell-y: "8px"
components:
  button-primary:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    rounded: "{rounded.control}"
    padding: "12px 20px"
  button-primary-disabled:
    backgroundColor: "{colors.rule}"
    textColor: "{colors.graphite}"
  button-secondary:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
    padding: "6px 14px"
  button-secondary-hover:
    backgroundColor: "{colors.paper-2}"
  button-destructive:
    backgroundColor: "{colors.brick}"
    textColor: "{colors.paper}"
    rounded: "{rounded.control}"
    padding: "8px 16px"
  segmented-selected:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    padding: "6px 12px"
  stamp:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    typography: "{typography.stamp}"
    padding: "0 6px"
  key-tag:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.key}"
    height: "18px"
    width: "18px"
  dialog:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.dialog}"
    width: "480px"
---

# Design System: Headroom

## Overview

**Creative North Star: "Planta Baixa"**

The disk is drawn as an architectural floor plan. Each project is a room whose area is its real recoverable space, partitioned into alcoves, one per artifact, and every area is given in GB as a dimension. The plan stands in for the category default of a rainbow sunburst or a treemap next to a table. Everything else on the screen is drafting-sheet furniture: a plain top bar, a grouped schedule (the list), and a dimension line in the footer.

The material is drafting paper and ink. The ground is an off-white sheet. Walls are black ink, getting thinner as they go from the outer boundary to the hairline partitions. Notes are graphite. Drafting blue is the one colored ink, and it only marks the selection. Depth comes from line weight and fill pattern (hatch and poché), never from shadow. The world is light only: it was chosen for a daytime desk.

The personality is trustworthy and precise. Figures are set in tabular Barlow and always shown in full. Ornament is limited to what a draftsman would actually draw.

**Key Characteristics:**
- Area equals real space; the plan is the data, not a decoration.
- Line weight carries hierarchy: 4px boundary, 3px room walls, 1px alcove partitions.
- One blue, one meaning: selected.
- Fill patterns (hatch, poché) carry state instead of color fills.
- Barlow throughout, tabular figures, plan lettering in tracked capitals.
- No shadows, no gradients as surface, no cards.

## Colors

A drafting sheet: near-white paper, black ink, graphite notes, one blue pencil, and one red reserved for deletion.

### Primary
- **Drafting Blue** (draft): the selection and nothing else. It is the diagonal hatch on selected alcoves, the filled checkbox (via accent-color), the selected span of the dimension line, and the focus ring. Hover never uses it.
- **Blue Tint** (draft-tint): the ground under the selection hatch, and text selection.

### Secondary
- **Brick** (brick): used only on the final destructive confirm button inside the delete dialog. It appears nowhere before that point.

### Neutral
- **Drafting Paper** (paper): page ground, alcove floors, label plates under plan lettering, dialog surface.
- **Paper Shade** (paper-2): hover ground for list rows and secondary buttons, and hairlines between artifact rows.
- **Ink** (ink): walls, primary text, the primary button, the selected segment, the stamp and key-tag borders, and the 1.5px rules under the table head and over the footer.
- **Graphite** (graphite): annotations such as paths, rules, ages, column labels, and secondary measures.
- **Rule Grey** (rule): 1px dividers (top bar, project group rows, dialog sections), control borders at rest, and the disabled primary button.
- **Poché Line** (poche-line): the fine horizontal rule fill inside active-project alcoves (1px every 6px).
- **Poché Swatch Line** (poche-swatch-line): a denser, darker poché (1px every 3.5px), used only for the legend sample so the pattern still reads at 14px.

### Named Rules
**The One Blue Rule.** Drafting blue means "selected" and nothing else. Hover, emphasis, links, and branding never borrow it. The one exception is the logo, a disk gauge whose blue layer is the space being freed (ink below is what stays, the dashed line on top is the headroom left). Its source is `app-icon.svg`, which generates every app and tray icon.

**The Last Door Rule.** Brick appears only on the button that performs an irreversible delete, inside the confirmation dialog. The primary "Limpar" action stays ink.

## Typography

**Display Font:** Barlow (with system-ui, sans-serif), self-hosted via @fontsource at 400/500/600/700
**Body Font:** Barlow
**Label/Mono Font:** Barlow (no mono)

**Character:** A single engineered grotesque, set like plan lettering. Names are tracked capitals, areas are set semibold, and all figures are tabular (`font-variant-numeric: tabular-nums` on body).

### Hierarchy
- **Wordmark** (700, 19px, +0.01em): the top-bar product name beside the mark.
- **Headline** (700, 20px): dialog titles.
- **Room Area** (600, 24px for rooms wider than 220px and taller than 120px, 16px otherwise; line-height 1.15): the GB dimension lettered inside a room, under its name.
- **Room Name** (700, 13px/18px, +0.05em, uppercase): plan lettering for rooms. The list repeats it for project group rows, in bold uppercase at body size.
- **Body Lead** (400, 15px): summary sentences, dialog copy, and primary button text. The plan total is set bold at 17px within it.
- **Body** (400, 14px): list rows and dialog list. Artifact names are 500, sizes in project rows are 700.
- **Annotation** (400, 13px): paths, legend entries, dimension-line measures, and secondary "de X" figures, in graphite.
- **Label** (600, 12px, +0.06em, uppercase): table column heads, the plan caption, and dialog column heads, in graphite.
- **Stamp** (700, 11px/16px, +0.1em, uppercase): the ATIVO stamp only.

### Named Rules
**The Whole Figure Rule.** GB figures are never ellipsized or truncated. If a figure doesn't fit, the label moves (key tag, legend, callout); the number keeps all its digits.

**The Tabular Rule.** All figures are tabular, so columns of sizes align and live re-measures don't jitter.

## Layout

The page is one full-height grid of four rows: a 56px top bar, the plan at `minmax(240px, 46%)`, the scrolling grouped list, and an 84px footer. There is a single column with 32px side gutters, and no breakpoints, since this is a desktop window (reviewed at 1280 and 1024 wide). The plan is laid out by squarified treemap, first into project rooms and then into artifact alcoves, both sized by the current mode (Real or Aparente).

The list is a schedule, not cards. Each project is its own table body: a group row (checkbox, uppercase name, graphite path, stamp, bold total), followed by artifact rows with 8px vertical cell padding. The table head is sticky over a 1.5px ink rule. The plan and the list mirror each other: hovering or selecting either one highlights the other.

The footer holds the dimension line, which stretches across, with the primary button at its right.

## Elevation & Depth

Flat. There are no box-shadows anywhere. Depth and hierarchy are drawn with line weight (4px, 3px, 1.5px, 1px) and fill pattern. The only overlay is the native dialog, which lifts by a 35% ink backdrop (`rgb(26 26 26 / 0.35)`) and a 1.5px ink border, not by a shadow.

### Named Rules
**The Line Weight Rule.** Hierarchy is drawn with line weight: 4px for the plan boundary, 3px between rooms, 1px hairlines between alcoves, and 1.5px ink for structural rules (table head, footer, stamp, key tag, dialog).

**The Pattern-Not-Paint Rule.** State on the plan is a fill pattern on paper: blue diagonal hatch (135°, 1.25px every 8px) for selected, horizontal poché for active. Solid colored fills are not used on the plan.

## Shapes

The plan is orthogonal and square-cornered: rooms, alcoves, walls, and the dimension line have no radius. Controls take a soft 6px radius. The dialog uses 8px. Lettering plates inside the plan use a 2px radius. The key tag is the only circle, a reference bubble as on a drawing. The dimension line ends and divides with 45° slash ticks, never arrowheads.

## Components

### Buttons
Plain drafting-office controls: ink on paper, weight over color.
- **Shape:** gently rounded (6px).
- **Primary ("Limpar N GB"):** ink fill, paper text, 15px semibold, 12px by 20px padding. Hover goes to #333. Disabled is a rule-grey fill with graphite text, labelled with what to do ("Selecione o que limpar").
- **Secondary ("Escanear", "Cancelar"):** 1px ink border on paper, semibold. Hover goes to the paper-2 ground.
- **Destructive:** brick fill, paper text, hover #74210f. Only in the confirm dialog, always paired with a focused Cancelar.
- **Focus:** a 2px drafting-blue outline at 2px offset, global.

### Segmented Toggle (Real / Aparente)
A single 1px ink outline at 6px radius. The selected segment is solid ink with paper text. The unselected segment is paper with a paper-2 hover. 13px medium. Implemented as role radiogroup.

### Plan (signature)
- **Boundary:** a 4px ink border. The ink ground shows through as walls: 3px between rooms and 1px between alcoves.
- **Room lettering:** a paper plate (2px radius) at the top-left holding the uppercase name over the area (24px or 16px). An active room adds an ATIVO stamp beside its name.
- **Key tag:** when the lettering doesn't fit, the room shows a circled number, plus its area if that fits. The legend under the plan explains it as number, name, and area.
- **Callout:** a room too small even for the tag (under 30px on a side) gets the tag outside the room, joined by a 1.25px ink leader line that ends in a 2px dot inside the room.
- **Alcove:** paper floor, labelled with artifact name (12px, graphite) over size (14px, semibold) when larger than about 84 by 40px. Selected shows the blue hatch. An active project shows poché. Hover shows a 2px ink outline inset 4px, never blue.
- **Repeated names:** projects with the same folder name, such as git worktrees, add parent folders until the name is unique, for example `web/main` and `api/feat/controle-pedra`. This applies on the plan, in the list and in the dialog.
- **Others room:** the plan draws at most 24 projects, and only those at or above 1% of the total. All the rest are pooled into one paper-2 room, lettered "+N projetos" over its area. It has no alcoves and no selection, because every project is still in the list.
- **Legend:** hatch swatch "Selecionado para limpar", poché swatch "Ativo · últimos 7 dias", then the keyed rooms, then a paper-2 swatch "+N projetos menores · área · na lista" when the others room exists.

### ATIVO Stamp
A rubber-stamp mark for projects modified in the last 7 days: 1.5px ink border, paper ground, 11px bold capitals tracked at 0.1em, no radius. Used in plan lettering, list group rows (with age), and the dialog warning.

### Dimension Line (signature)
A 1px ink line with 45° slash terminations at both ends and at the selection boundary. The selected span is overdrawn as a 3px drafting-blue bar. Each measure is centered over its own span, clamped to stay on screen: the selected measure is bold ink, the remainder is graphite. A graphite caption below names the whole. Bar, ticks, and labels move with a 300ms ease-out as the selection changes.

### List (schedule)
Checkboxes are 16px, native, with a drafting-blue accent, and are indeterminate for partly selected groups. Group rows sit over a 1px rule-grey line. Artifact rows sit over paper-2 hairlines. Hovering a row gives a paper-2 ground. In Real mode, shared hardlinks add a graphite "de X" apparent figure.

### Dialog
Native `dialog`, 480px, 8px radius, 1.5px ink border, paper. A 20px bold title asks the question with the count. Graphite copy states the real space freed and that nothing goes to the trash. It contains a bordered list of folders with real sizes, an ATIVO warning when active projects are included, and a footer, separated by a rule, holding Cancelar (autofocus) and the brick confirm.

## Do's and Don'ts

### Do:
- **Do** reserve drafting blue (#2f5d8a) for selection: hatch, checkbox, the dimension-line span, and the focus ring.
- **Do** show hover with ink: a 2px inset ink outline on the plan and a paper-2 ground in the list.
- **Do** draw walls at 4px (outer), 3px (between rooms), and 1px (between alcoves).
- **Do** letter every room: name over area, then a circled key tag, then a callout with a leader line, in that order of fallback.
- **Do** mark active projects with poché plus the ATIVO stamp, and keep the legend swatch denser and darker so it reads at small size.
- **Do** set every figure in tabular Barlow and show it whole.
- **Do** end dimension lines with 45° slashes and center each measure over its own span.

### Don't:
- **Don't** ellipsize or truncate a GB figure.
- **Don't** use blue for hover, links, or emphasis.
- **Don't** use brick anywhere except the final destructive confirm.
- **Don't** add shadows, gradient surfaces, or cards; depth is line weight and pattern.
- **Don't** use solid colored fills for state on the plan; use hatch or poché.
- **Don't** add a dark theme; the world is light only, for a daytime desk.
