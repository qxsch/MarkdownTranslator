# I finally ditched my flaky CI pipeline 🎉

Okay, confession time: for months our build pipeline was a total mess. Every other run blew up for no reason, and we all just hit "re-run" and crossed our fingers. Sound familiar? Yeah, I thought so.

Last week I finally had enough and sat down to fix it for good. Spoiler: it wasn't rocket science, but it took a bit of detective work. Here's what I learned, so you don't have to go down the same rabbit hole.

## The usual suspects

First things first, I rounded up the usual suspects:

- **Tests that depend on each other.** One test leaves junk in the database, and the next one trips over it. Classic.
- **Timeouts that are way too tight.** Our shared runners are slow on Monday mornings (everybody's pushing at once), so a 30-second limit was basically a coin flip.
- **Caches nobody understood.** We cached `node_modules` with a key that never changed, so stale packages kept sneaking back in.

Honestly, the cache thing was the real kicker. Once I changed the key to include the hash of `package-lock.json`, half of the random failures just vanished into thin air.

## Making it stick

Fixing stuff is one thing, keeping it fixed is another. So I added a tiny script that flags any test that fails and then passes on retry:

```python
# Flag tests that only pass on the second attempt
def is_flaky(results):
    return results[0] == "failed" and results[-1] == "passed"  # classic flake
```

Now flaky tests show up in a dashboard instead of quietly wasting everyone's time. Peer pressure works wonders, by the way: nobody wants their name next to the flakiest test of the week. 😅

## Was it worth it?

Absolutely. Our success rate went from roughly 70% to over 98%, and people actually trust the red and green lights again. If your pipeline is driving you up the wall, give these tricks a shot and let me know how it goes!
