# Phase 45 — In-app Purchase / Store Placeholder

> **REFERENCE-ONLY UI.** A "Pro features unlocked" / paywall surface in case
> a future FCast Android variant ever wants per-tier features. **Never
> instantiated by `MainWindow`.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3
**Functional integration:** **Never wired.** Lives under
`senders/android/ui/pages/_apple/` next to other reference-only surfaces.

**Moblin source analogues** (1 file):
- `View/Settings/Cosmetics/CosmeticsSettingsView.swift`

**Files to add:**
- `senders/android/ui/pages/_apple/cosmetics_store_page.slint`

---

## Goal

Render Moblin's "store" surface as a placeholder. Pure reference — FCast
has no IAP and is not planning to ship one.

---

## Moblin source pattern

```swift
// CosmeticsSettingsView.swift (excerpt)
List {
    ForEach(productGroups) { group in
        Section {
            ForEach(group.products) { product in
                HStack {
                    Image(product.iconName); Text(product.title); Spacer()
                    if product.owned { Image(systemName: "checkmark") }
                    else { Button(product.priceLabel) { /* purchase */ } }
                }
            }
        } header: { Text(group.title) }
    }
}
```

---

## Tasks

### 45-A — `CosmeticsStorePage`

```slint
export struct StoreProduct {
    id: string,
    title: string,
    description: string,
    price-label: string,        // localized "$1.99"
    owned: bool,
}

export struct StoreSection {
    title: string,
    products: [StoreProduct],
}

export component CosmeticsStorePage inherits Rectangle {
    in-out property <[StoreSection]> mock-sections: [
        { title: "Themes", products: [
            { id: "theme-night",  title: "Night theme",  description: "Pure black UI",
              price-label: "$0.99", owned: true  },
            { id: "theme-mint",   title: "Mint theme",   description: "Pastel mint accents",
              price-label: "$0.99", owned: false },
        ]},
        { title: "Icons", products: [
            { id: "icon-classic", title: "Classic icon", description: "FCast 2024 art",
              price-label: "$0.99", owned: false },
            { id: "icon-pride",   title: "Pride icon",   description: "Rainbow gradient",
              price-label: "$0.99", owned: false },
        ]},
    ];

    background: Theme.surface-primary;
    VerticalLayout {
        padding: Theme.padding-screen; spacing: Theme.spacing-md;

        Text { text: "Store"; font-size: Theme.font-size-title;
               font-weight: FontWeight.semi-bold; color: Theme.text-primary; }

        for sec in root.mock-sections: SettingsSection { title: sec.title;
            for product in sec.products: HorizontalLayout {
                spacing: 12px;
                Rectangle {
                    width: 48px; height: 48px;
                    background: Theme.surface-card;
                    border-radius: 8px;
                    Text { text: "🎨"; font-size: 24px;
                           horizontal-alignment: center;
                           vertical-alignment: center; }
                }
                VerticalLayout {
                    horizontal-stretch: 1;
                    Text { text: product.title; color: Theme.text-primary;
                           font-weight: FontWeight.semi-bold; }
                    Text { text: product.description; color: Theme.text-secondary;
                           font-size: Theme.font-size-caption; }
                }
                if product.owned:
                Text { text: "✓ Owned"; color: Theme.success;
                       vertical-alignment: center; }
                if !product.owned:
                PrimaryButton { label: product.price-label; }
            }
        }
    }
}
```

---

## Exit criteria

1. `CosmeticsStorePage` renders 2 sections × 2 products with owned /
   "$0.99 buy" state.
2. Lives under `_apple/`; never imported by `main.slint`.
3. `cargo build -p android-sender` passes (no-op since not imported).

---

## What's NOT in this phase

- Real Play Billing / StoreKit / receipt validation.
- Real entitlement gating in the rest of the UI.
- Restore Purchases flow.

---

## Slint best practices applied here

- **Nested `for` over `mock-sections.products`** is supported in Slint —
  inner iteration captures the outer `sec` cleanly.
- **`if product.owned: ... if !product.owned:` pair** keeps the price
  button and "✓ Owned" badge mutually exclusive without any imperative
  visibility flip.
