# Lane — what the catch-up's disk drawdown actually scales with

Owner: session `a91c2501` (generation 5 coordinator), 2026-08-16.
Question: **will the large tail batches run the disk through cass's ~32 GiB
startup floor and stop the catch-up one batch from done?**

Answer: **no.** Peak transient drawdown is dominated by a fixed ~2 GiB cost and
does not scale with batch size. Measured, not fitted.

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

**Source rose 18.6x. Peak did not rise.** It stayed in a 1.5–4.9 GiB band the
whole way, mean 2.23 GiB, and the ratio column collapsed monotonically from 21.6x
to 3.5x — which is what a constant divided by a growing number does. That is the
whole finding: the cost is per-batch overhead, not per-byte.

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

Batch 27 is 5.49 GiB of source, batch 28 is 1.81 GiB. Expected transient for
each is the fixed ~2–5 GiB, not the ~13 GiB the old slope predicted. Free space
is 57 GiB against a ~32 GiB floor. The tail clears it with room, and would have
cleared it even without the disk cleanup — the cleanup's real value was removing
the *permanent* archive-growth squeeze (1.75x source, unbounded across batches),
which is a different quantity and remains the one worth watching.

## Method note worth keeping

The first version of this measurement asked the volume how much space a batch
used. Free space on this machine moves under ~20 concurrent agent sessions, so it
answered with their noise. Sampling each batch against **its own** starting free
space, and taking a peak rather than an endpoint, separates transient from
permanent and from everyone else's activity. Same lesson as the 5.67x-vs-1.75x
archive-expansion error earlier in this repair: measure the subject, not the
volume it sits on.
