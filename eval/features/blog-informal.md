# Why we rewrote our sync engine (and what went wrong)

Let's be honest: the old engine was a pain. It worked, sort of, but every new feature felt like pulling teeth. So last spring we bit the bullet and started from scratch.

## The first attempt was a flop

We thought we'd nail it in three months. Spoiler: we didn't. Our first prototype was blazing fast on our laptops and fell apart the moment we threw real customer data at it. Folders with 200,000 files? Game over.

## What finally clicked

The breakthrough came when we stopped comparing files one by one and started comparing hashes of whole folders. If the hash matches, we skip the folder. Simple, right? It took us way too long to see it.

## Numbers, because you asked

- Initial scan of 1 million files: 41 minutes before, 6 minutes now
- Memory use: down from 1.2 GB to 300 MB
- Crash reports: down by 80%

## What's next

We're not done yet. Next up is peer-to-peer sync on the local network, so your files don't take a round trip through the cloud when your colleague sits right next to you. Stay tuned, and if you hit a bug, give us a shout!
