# Lane — what the catch-up's disk drawdown actually scales with

Owner: session `a91c2501` (generation 5 coordinator), 2026-08-16.
Question: **will the large tail batches run the disk through cass's ~32 GiB
startup floor and stop the catch-up one batch from done?**

Answer: **no — but not for the reason this note originally gave.** The run is
safe because the disk was cleared, not because drawdown is size-independent.

> ## CORRECTION, 2026-08-16T14:48Z — the original conclusion below was WRONG
>
> This note first concluded that peak drawdown is a **fixed ~2 GiB cost that does
> not scale with batch size**, and predicted ~2–5 GiB for the 5.5 GiB tail batch.
> `batch-ba` then ran with 5.519 GiB of source and peaked at **9.765 GiB**.
>
> The fixed-cost reading held across 0.075–1.395 GiB and broke immediately past
> it. There *is* a source-proportional term; it was simply swamped by fixed
> overhead everywhere the sampling could see. Across both regimes the honest fit
> is **`peak ≈ 1.95 GiB + 1.42x source`**.
>
> **The failure is the one this note was written to criticize.** It faults the
> earlier model for fitting a 3.5x range and extrapolating twenty times past it,
> then extrapolates four times past its own largest point and lands wrong in the
> same direction. Twenty-two points across an 18.6x source range still could not
> see a term that only dominates beyond where they stop. Flatness inside a
> sampled window is not evidence of flatness outside it, however many points the
> window holds.
>
> **The original 2.1x-per-byte model was the better one to plan on.** It predicted
> 13.3 GiB against an actual 9.765 — conservative by 3.5 GiB and correct in kind.
> The disk cleanup was therefore load-bearing for the *transient* spike too, not
> only for permanent archive growth as the closing section below claims.
>
> Everything under "The measurement" is raw data and stands. The verdict drawn
> from it does not.

## Why this was asked at all

The projection that drove the disk cleanup used `1.68 GiB fixed + 2.1x source`,
fitted across 14 batches whose sources spanned only 0.075–0.263 GiB — a 3.5x
range. Batch 27 is **5.49 GiB**, twenty times the largest point in that fit.
Extrapolating a linear model twenty times past its data is not evidence, and the
`2.1x` slope predicted ~13 GiB of transient for that batch. A single-point ratio
from an early batch (21.5x) predicted **118 GiB**, which would have been fatal.

So a waiter sampled every batch as it completed and held its verdict until the
source range reached 3x discrimination.

## The measurement

Sampled at 5s intervals through each batch; peak drawdown is the largest
free-space deficit against that batch's own starting free space, so permanent
archive growth is excluded.

| batch | source GiB | peak drawdown GiB | ratio |
|---|---|---|---|
| `batch-ae` | 0.075 | 1.619 | 21.6x |
| `batch-af` | 0.080 | 1.520 | 19.0x |
| `batch-ag` | 0.084 | 1.863 | 22.1x |
| `batch-ah` | 0.089 | 2.631 | 29.6x |
| `batch-ai` | 0.094 | 1.951 | 20.7x |
| `batch-aj` | 0.099 | 1.839 | 18.5x |
| `batch-ak` | 0.106 | 1.766 | 16.7x |
| `batch-al` | 0.112 | 1.884 | 16.8x |
| `batch-am` | 0.121 | 1.951 | 16.2x |
| `batch-an` | 0.130 | 1.850 | 14.3x |
| `batch-ao` | 0.142 | 1.998 | 14.1x |
| `batch-ap` | 0.154 | 1.578 | 10.2x |
| `batch-aq` | 0.169 | 2.749 | 16.3x |
| `batch-ar` | 0.191 | 2.720 | 14.2x |
| `batch-as` | 0.224 | 2.203 | 9.8x |
| `batch-at` | 0.263 | 0.223 | 0.8x |
| `batch-au` | 0.317 | 2.806 | 8.9x |
| `batch-av` | 0.420 | 2.460 | 5.9x |
| `batch-aw` | 0.515 | 2.871 | 5.6x |
| `batch-ax` | 0.611 | 3.472 | 5.7x |
| `batch-ay` | 0.829 | 2.275 | 2.7x |
| `batch-az` | 1.395 | 4.916 | 3.5x |
| `batch-ba` | **5.519** | **9.765** | **1.8x** | *(added after the correction above)* |

**Within 0.075–1.395 GiB, source rose 18.6x and peak did not rise.** It stayed in
a 1.5–4.9 GiB band, mean 2.23 GiB, and the ratio collapsed monotonically from
21.6x to 3.5x — which is what a constant divided by a growing number does. That
reading was correct about the sampled window and false about the tail.

`batch-ba` is the point that breaks it: 4x the source of the largest prior batch,
and roughly 4x the peak with it. The ratio kept falling (1.8x) because the fixed
term keeps shrinking as a share — so *the ratio column never signalled the
breakdown*, and watching it was what made the flat reading look robust. A falling
ratio is compatible with a rising absolute cost, which is the quantity the disk
floor actually cares about.

## Read the fit's numbers as junk, and the verdict as sound

The waiter's own linear fit came out `peak = 2.22 GiB + -2.4 x source` and
projected **-11.1 GiB** for batch 27. A negative peak is physically impossible.
Fitting a line to a flat noisy band recovers the intercept correctly and the
slope arbitrarily, so the intercept (2.22 GiB, within 0.01 of the measured mean)
is the real quantity and the slope is noise that happened to land negative.

The verdict `FIXED` is what the data supports; the projected number printed next
to it is not. Both came out of the same script, and only one of them should be
quoted. That is the instrument-label failure in
`~/.agent-config/.claude/rules/instrument-labels.md` — a label asserting more
than the computation established — caught here before it reached a report.

## What it means for the tail

**Superseded — the original text of this section is kept below the line for the
record, and it is wrong.**

`batch-ba` (5.519 GiB) ran and peaked at 9.765 GiB, completing `rc=0` with free
space at 51 GiB. One batch remains: `batch-bb` at 1.81 GiB, which the corrected
model `1.95 + 1.42x source` puts at ~4.5 GiB peak against 51 GiB free and a
~32 GiB floor. It clears comfortably.

The claim that the tail "would have cleared even without the disk cleanup" is
**false**. At the pre-cleanup 45 GiB, `batch-ba` starting near the bottom of the
run would have taken free space to roughly the floor with permanent archive
growth stacked on top of a 9.765 GiB transient. The cleanup was load-bearing for
both quantities, not just for permanent growth.

---

*Original text, retained because it is what the record said and what any reader
before 14:48Z acted on:*

> Batch 27 is 5.49 GiB of source, batch 28 is 1.81 GiB. Expected transient for
> each is the fixed ~2–5 GiB, not the ~13 GiB the old slope predicted. Free space
> is 57 GiB against a ~32 GiB floor. The tail clears it with room, and would have
> cleared it even without the disk cleanup — the cleanup's real value was removing
> the *permanent* archive-growth squeeze (1.75x source, unbounded across batches),
> which is a different quantity and remains the one worth watching.

## The transferable lesson, rewritten after being wrong

The original version of this note ended with a method lesson about measuring the
subject rather than the volume. That lesson is sound and is kept below. But it is
not the lesson this exercise actually taught, because the method was fine and the
conclusion was still wrong.

**The real one: a model earns trust only inside the range it was fitted on, and
adding points inside that range does not extend it.** Twenty-two points felt like
overwhelming evidence and bought nothing at 5.5 GiB, because every one of them
sat where the fixed term dominated. The correct move, once the verdict was going
to be quoted about a batch 4x past the data, was to say so explicitly and plan on
the conservative model until a point in that regime existed — which is precisely
what the earlier `2.1x` model, for all its own over-extrapolation, delivered.

Second: **pick the statistic the decision depends on.** The ratio column fell
monotonically the whole way, including through the batch that broke the model,
so it could never have raised the alarm. The disk floor cares about absolute
GiB. Watching a normalized quantity while the decision turns on an absolute one
is how a metric stays reassuring through the event it was supposed to catch.

## Method note worth keeping

The first version of this measurement asked the volume how much space a batch
used. Free space on this machine moves under ~20 concurrent agent sessions, so it
answered with their noise. Sampling each batch against **its own** starting free
space, and taking a peak rather than an endpoint, separates transient from
permanent and from everyone else's activity. Same lesson as the 5.67x-vs-1.75x
archive-expansion error earlier in this repair: measure the subject, not the
volume it sits on.
