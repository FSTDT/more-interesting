# A kit for more interesting links and discussions

More Interesting (will become) a link sharing app that seeks to avoid discussion tangents, circlejerkery, and mobbing.
It's a bit like Reddit, and closer to [Lobsters](https://lobste.rs), but More Interesting tries to improve even more
on the link sharing network paradigm.

## Features

### Private, invite-only, fast

More Interesting is currently invite-only. I'm not sure if I want to stay that way forever, but at least for now...
 
More Interesting doesn't collect your email address, IP address, or other PII information at all unless you post it.

### Discussion and Ranking

The biggest difference from the mold is how More Interesting's alternatives to Threading and Voting.

For stories, the model is relatively similar: "Starring" a post causes it to go up, and "Reporting" a post causes it to
go down. Unlike in Lobsters and Reddit, though, a post that receives enough reports from a trusted user will
be completely hidden, regardless of how many stars it gets. Ten reports and twenty stars? Better hide it! Additionally,
More Interesting does not rank a story higher because of its comment count; ranking is exclusively based on star count
and time.

The conversation below, on the other hand, almost entirely avoids any form of ranking. The default view is
chronological, with no sorting whatsoever. In a conversation with more than one hundred replies,
MI offers an additional "summary" view which hides comments that received no stars.

The most influential change, though, is probably that stars are not anonymous. Flags are partially anonymous, since mods
can see them but other users cannot.

### Tagging

This part is partially copied from lobsters, with a few additional tweaks:

All posts need at least one tag and at most four.

Because More Interesting is designed to be a news site, with no specific "tech" focus, it allows tags to be completely
user-driven. There is also no penalty associated with particular tags, other than the fact that users can filter certain
tags out.

More Interesting additionally allows you to mark a post as something you authored, which gives it a small boost.

## Running locally

Start it up with these env variables set:

* `MORE_INTERESTING_INIT_USERNAME`
* `MORE_INTERESTING_INIT_PASSWORD`

Then go to `http://localhost:3001/setup`
