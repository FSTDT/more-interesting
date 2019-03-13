# A kit for more interesting links and discussions

More Interesting (will become) a link sharing app that seeks to avoid discussion tangents, circlejerkery, and mobbing.
It's a bit like Reddit, but closer to [Lobsters](https://lobste.rs), but More Interesting tries to improve even more
on the link sharing network paradigm.

A lot of this stuff isn't implemented, yet.

## Will-be features

### Transparency

Publicly-visible moderation logs are the starting point.

Who stars what should also be visible, both showing what a user stars on their profile page,
and offering a way to show who starred a post.
Stars would be less like Upvotes, and more like Likes.

Flags, on the other hand, would remain anonymous.
WAY too much opportunity for harassment in making those public.
As per MI's copying of the [Discourse](https://discourse.org/)
moderation model, flags are not anonymous to the moderators,
and they are intentionally designed to escalate quickly (three flags are enough to hide a post) and have
good feedback, so that users are encouraged to employ them,
and don't give up on them ["because it's ineffective"](https://lobste.rs/s/v9rktg/new_users_ethics_civility_threads_you#c_xm6qwm).

### Private, invite-only, fast

More Interesting is going to have an invite tree.
 
More Interesting doesn't require your email address, phone number, IP address, or other PII information.

More Interesting allows users to participate without JavaScript. While the vote buttons probably will use JS, most other forms of interaction shouldn't need it. See [details-dialog](https://github.com/github/details-dialog-element), the menues in [bors-ng](https://app.bors.tech), and [this post](https://dev.to/winduptoy/a-javascript-free-frontend-2d3e).

And [this list. So much this list!](https://axesslab.com/accessibility-according-to-pwd/)

More Interesting displays 200 articles on the front page. This is in stark contrast to other sites,
which typically only show 25 articles on the front page. Pulling this off requires, and will continue to require,
careful benchmarking and frontend design.

### Discussion and Ranking

The biggest difference from the mold is how More Interesting's alternatives to Threading and Voting.

For stories, the model is relatively similar: "Starring" a post causes it to go up, and time causes a post to
go down. Unlike in Lobsters and Reddit, though, a post that receives enough reports from trusted users will
be completely hidden, regardless of how many stars it gets. Three flags and twenty stars? Better hide it automatically!
Additionally, More Interesting does not rank a story higher because of its comment count; ranking is exclusively based
on star count and time.

The comment conversation, on the other hand, almost entirely avoids any form of ranking. The default view is
chronological, with no other sorting whatsoever. In a conversation with more than one hundred replies,
MI offers a non-default "summary" view which hides comments that received few stars (the heuristic is pending).
Comments are also presented in chronological order, without nested presentation.
Clicking the "reply" button on a comment should add a `#12345` thing to the comment
that should link back to the original one.

The most influential change, though, is probably that stars are not anonymous. Flags are partially anonymous, since mods
can see them but other users cannot.

So, basically, like this:

    Home        user [+ POST] [LOG OUT]
    -----------------------------------
    
    _Bitcoin is the get-rich-quick scheme that might actually work_
    180 [⭐] [🚩] submitted by @dewey | 1h ago | 2K comments
    
    -----------------------------------
    
    [✔] hide comments with less than 10 stars
    
    -----------------------------------
    
    I'm not much of a techie, but isn't that kind of economically
    impossible? And it's not even really in the spirit of the
    original paper?
    
    -- 215 [⭐] [🚩] @webby 1hr ago [REPLY]
    
    -- [+] 1 comment hidden --
    
    @webby #12345
    
    > isn't that kind of economically impossible
    
    There's no such thing as "economically impossible."  Money
    is all made up, remember? It's not like it's physics.
    
    > not even really in the spirit of the original paper
    
    Why are you treating some anonymous Japanese guy's word as gospel?
    
    -- 100 [⭐] [🚩] @dewey 1hr ago [REPLY]
    
    -- [+] 13 comments hidden --
    
    @bubba #23456
    
    Who allowed YOU near a computer?! WHO EVEN THOUGHT IT WAS A GOOD IDEA
    TO PUT YOU INTO A COMMENT THREAD THAT WAS OBVIOUSLY BASED ON THE REBOOT?!
    
    @notriddle
    
    Why was this guy even invited onto this instance?
    
    -- 1K [⭐] [🚩] @huey 1hr ago [REPLY]

There are several reasons for choosing this design for comments:

* When possible, I want to encourage people to just read the entire thread. Reading is fundamental. Discussion threads are stories, and thus, should be presented in chronological order (or, at least, with only a small sprinkling of explicitly "en media res").
* I want to avoid giving people ways to strategically subvert the Reply model. Nested threads like Reddit and Lobsters encourage people to reply to the currently-top megathread, instead of making their own separate replies. This makes the non-threaded reply mechanism a trap for the inexperienced, assuming they want their not-very-upvoted thread to be seen instead of just being hidden behind the massively-upvoted megathread.
* Starring comments helps to quench the "me-too" impulse without crowding it out. Also, the star counts ("1K", "100", "215", and "180") should all be clickable links to shown who voted a certain way.
* And, of course, by not ranking posts by comment count (only stars and the "authored by" bit act as boosts), I try to avoid creating megathread "brush fires," or worse, situations where attempting to decry something actually boosts its visibility. Telling someone that their idea sucks should not cause the post to float to the top, and in the absence of good sentiment analysis, I'd rather do nothing.

### Tagging

All posts need at least one tag and at most four.

Because More Interesting is designed to be a news site, with no specific "tech" focus, it allows tags to be completely
user-driven. There is also no penalty associated with particular tags, other than the fact that users can filter certain
tags out.

More Interesting additionally allows you to mark a post as something you authored, which gives it a small boost.

Copying hats from lobsters (essentially, allowing someone to tag their post with a flair who's authenticity was vetted by the mods) also seems like a good idea.

### Plain text

Here's another spot where I'm deviating, not just from the Reddit mold, but from Reddit, Lobsters, and even Discourse.
Not using Markdown.

Twitter did it right. Nobody ever has to ask how to hashtag something, or how to mention someone. If you see someone else do it, you can just imitate them.
And since the syntax is so minimal, you're unlikely to trigger it by mistake.
Power of plain text in action.

More Interesting should do the same thing. A quick sketch of what the comment form should look like:

    -------------------------------------------------------------------------
    | Write my comment here                                                 |
    |                                                                       |
    | @mentioning someone will ping them, just like on Twitter              |
    |                                                                       |
    | #12345 numbers will link to another comment on the same post          |
    |                                                                       |
    | #words are hashtags, just like Twitter                                |
    |                                                                       |
    | Consecutive line breaks are paragraph breaks, like in Markdown        |
    |                                                                       |
    | URL's are automatically linked, following the GitHub-flavor MD rules  |
    | <URL> is a usable workaround if your URL is too complex, but note that|
    | the angle brackets will still be shown! We also include GitHub's WWW  |
    | special case, like www.example.com, <www.example.com>,                |
    | http://example.com, and <http://example.com> will all work.           |
    |                                                                       | 
    |        Multiple spaces in a row are stripped.  That way, people who   |
    | still do double-space after a full stop, or who insist on indenting   |
    | paragraphs, don't accidentally trigger code mode or something.        |
    |                                                                       |
    | And that's it. There are no headings, there are no code mode, not even|
    | backslash escaping.                                                   |
    -------------------------------------------------------------------------
    
    [SUBMIT] [+ ATTACHMENT]

Note that the comment box should start out fairly large, to encourage long-form writing,
and will expand as the user writes.

Anyway, the comment shown there should render into this HTML:

```html
<p>Write my comment here</p>
<p><a href="mentioning">@mentioning</a> someone will ping them, just like on Twitter</p>
<p><a href="#12345">#12345</a> numbers will link to another comment on the same post</p>
<p><a href="?tag=words">#words</a> are hashtags, just like Twitter</p>
<p>Consecutive line breaks are paragraph breaks, like in Markdown</p>
<p>URL's are automatically linked, following GitHub-flavor MD rules &lt;URL&gt; is a usable workaround if your URL is too complex, but note that the angle brackets will still be shown! We also include GitHub's WWW special case, like <a href="http://www.example.com">www.example.com</a>, <a href="http://www.example.com">&lt;www.example.com&gt;</a>, <a href="http://example.com">http://example.com</a>, and <a href="http://example.com">&lt;http://example.com&gt;</a> will all work.</p>
<p>Multiple spaces in a row are stripped. That way, people who still do double-space after a full stop, or who insist on indenting paragraphs, don't accidentally trigger code mode or something.</p>
<p>And that's it. There are no headings, there are no code mode, not even backslash escaping.</p>
```

Attachments can be used to add images (since, as you've noticed, there's no image syntax) and text files (which, unlike the contents of a comment itself, will be rendered with a monospace font and in `<pre>`-formatted form).

In practice, I expect the same conventions as on Twitter and Hacker News will be employed (`as shown by research [1]\n\n[1]: http://arxiv.org/whatever`).

Also, the system should use global IDs for comments, and it should ensure that the comment exists before linking to it.
In other words, I don't want `#1` to get auto-linked, so comment `#1` should only be the first comment ever made, not the first one in the thread.

## Running locally

Run it with Cargo with these env vars set:

* `MORE_INTERESTING_INIT_USERNAME`
* `MORE_INTERESTING_INIT_PASSWORD`

Then go to `http://localhost:3001/-setup`

## License

For now, I'm going with the same license terms as Rust itself:

You can redistribute this software under the terms of either the Apache license,
in [LICENSE-APACHE](LICENSE-APACHE), or the MIT license in [LICENSE-MIT](LICENSE-MIT),
at your option. By contributing to this repository, you agree to license your
contributions under these same terms.

I might be talked into switching to the AGPL or something, but for now I'd rather
leave my options open (the MIT-licensed code is compatible with the AGPL anyway).
