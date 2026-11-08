# gymshark-cli

Browse Gymshark from the terminal: list collections, search, find deals, read reviews, and check stock for a specific size. It needs no account, no API key, and no dependencies beyond Python 3.9+.

```
$ gymshark ls shorts mens --size m --in-stock --sort rating --limit 3
Shorts (mens): 297 products over 5 pages; 57 match on page 1, showing 3

Arrival 5" Shorts — Black          $26  XS S M L XL XXL 3XL  4.2★ (1194)
    gymshark-arrival-5-shorts-black-ss22
...
```

## Install

```
pip install ./gymshark-cli        # gives you the `gymshark` command
# or run without installing:
python3 -m gymshark_cli --help    # from inside gymshark-cli/
```

## Commands

| Command | What it does |
|---|---|
| `gymshark categories [mens\|womens]` | Lists the collection handles you can browse. |
| `gymshark ls <collection> [mens\|womens]` | Lists a collection, 60 items per page. Use `--page N`, `--pages N`, or `--all` to see more. |
| `gymshark find <words…> [mens\|womens]` | Keyword search. Every word must match the title, colour, type, fit, range, or features. |
| `gymshark deals [mens\|womens]` | Shows discounted items from Last Chance, biggest discount first. |
| `gymshark product <handle-or-url> [-d]` | Shows price, stock per size, review breakdown, other colours, and "get the look". |
| `gymshark stock <handle-or-url> [--size M] [--watch 900]` | Checks stock. Exits 0 if in stock and 1 if not, so you can use it in scripts. |

Filters for `ls`, `find`, and `deals`: `--size`, `--colour`, `--min-price`, `--max-price`, `--min-rating`, `--sale`, `--in-stock`, `--sort {price,-price,discount,rating,reviews}`, `--limit`.

Every command accepts `--json` for machine-readable output and `--fresh` to skip the 10-minute cache.

### Examples

```
gymshark find seamless leggings womens --size s --sort rating
gymshark deals womens --size m --max-price 30
gymshark product https://www.gymshark.com/products/gymshark-arrival-5-shorts-black-ss22
gymshark stock gymshark-arrival-5-shorts-black-ss22 --size xl && echo "go buy it"
gymshark ls joggers mens --all --json | jq '.products[] | select(.discount > 0) | .title'
```

Get alerted when a sold-out size comes back. Either run `--watch` in a terminal, or use cron:

```
*/30 * * * * gymshark stock <handle> --size m -q || true   # exit 0 = back in stock
```

### Notes on the ratings sort

`--sort rating` weights each average by its review count: it pulls an item's score toward 4.0 until it has a few dozen reviews. This keeps a 5.0★ item with 1 review from outranking a 4.6★ item with 900 reviews.

## How it works, and its limits

Gymshark runs a headless Shopify store on Next.js. Each collection and product page embeds its data as JSON in a `<script id="__NEXT_DATA__">` tag, and this tool reads that JSON. There is no official public API, so:

- **The tool can break when Gymshark changes its site.** When that happens, it shows a clear error saying no page data was found.
- **It is read-only on purpose.** It never touches the bag, checkout, or your account. Those paths are disallowed in Gymshark's `robots.txt`, and automating checkout would put you in bot territory.
- **It does not use `/search`**, because `robots.txt` disallows it. `find` scans `all-products` instead. That takes about 17 requests the first time and is instant for the next 10 minutes because of the cache.
- **It does not get around waiting rooms.** During drops, Gymshark sends visitors to a Queue-it page. The CLI tells you this and stops.
- **It is polite to the site.** It identifies itself honestly in its User-Agent, uses gzip (250 KB per page instead of 2.2 MB), waits 1 second between uncached requests, and won't let `--watch` poll faster than every 5 minutes.
- **It uses the US store and shows prices in USD.** To try another region, set `GYMSHARK_BASE_URL` (for example `https://uk.gymshark.com`). That is untested: regional sites may differ.

Cache location: `$XDG_CACHE_HOME/gymshark-cli`, or `~/.cache/gymshark-cli` if that isn't set.

## Tests

```
cd gymshark-cli && python3 -m unittest discover -s tests
```

The tests run offline against trimmed copies of real pages in `tests/fixtures/`.
