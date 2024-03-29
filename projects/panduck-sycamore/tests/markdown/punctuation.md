## Smart punctuation

Open quotes are matched with closed quotes.
The same method is used for matching openers and closers
as is used in emphasis parsing:

```````````````````````````````` example
"Hello," said the spider.
"'Shelob' is my name."
.
<p>“Hello,” said the spider.
“‘Shelob’ is my name.”</p>
````````````````````````````````

```````````````````````````````` example
'A', 'B', and 'C' are letters.
.
<p>‘A’, ‘B’, and ‘C’ are letters.</p>
````````````````````````````````

```````````````````````````````` example
'Oak,' 'elm,' and 'beech' are names of trees.
So is 'pine.'
.
<p>‘Oak,’ ‘elm,’ and ‘beech’ are names of trees.
So is ‘pine.’</p>
````````````````````````````````

```````````````````````````````` example
'He said, "I want to go."'
.
<p>‘He said, “I want to go.”’</p>
````````````````````````````````

A single quote that isn't an open quote matched
with a close quote will be treated as an
apostrophe:

```````````````````````````````` example
Were you alive in the 70's?
.
<p>Were you alive in the 70’s?</p>
````````````````````````````````

```````````````````````````````` example
Here is some quoted '`code`' and a "[quoted link](url)".
.
<p>Here is some quoted ‘<code>code</code>’ and a “<a href="url">quoted link</a>”.</p>
````````````````````````````````

Here the first `'` is treated as an apostrophe, not
an open quote, because the final single quote is matched
by the single quote before `jolly`:

```````````````````````````````` example
'tis the season to be 'jolly'
.
<p>’tis the season to be ‘jolly’</p>
````````````````````````````````

Multiple apostrophes should not be marked as open/closing quotes.

```````````````````````````````` example
'We'll use Jane's boat and John's truck,' Jenna said.
.
<p>‘We’ll use Jane’s boat and John’s truck,’ Jenna said.</p>
````````````````````````````````

An unmatched double quote will be interpreted as a
left double quote, to facilitate this style:

```````````````````````````````` example
"A paragraph with no closing quote.

"Second paragraph by same speaker, in fiction."
.
<p>“A paragraph with no closing quote.</p>
<p>“Second paragraph by same speaker, in fiction.”</p>
````````````````````````````````

A quote following a `]` or `)` character cannot
be an open quote:

```````````````````````````````` example
[a]'s b'
.
<p>[a]’s b’</p>
````````````````````````````````

Quotes that are escaped come out as literal straight
quotes:

```````````````````````````````` example
\"This is not smart.\"
This isn\'t either.
5\'8\"
.
<p>&quot;This is not smart.&quot;
This isn't either.
5'8&quot;</p>
````````````````````````````````

Two hyphens form an en-dash, three an em-dash.

```````````````````````````````` example
Some dashes:  em---em
en--en
em --- em
en -- en
2--3
.
<p>Some dashes:  em—em
en–en
em — em
en – en
2–3</p>
````````````````````````````````

A sequence of more than three hyphens is
parsed as a sequence of em and/or en dashes,
with no hyphens. If possible, a homogeneous
sequence of dashes is used (so, 10 hyphens
= 5 en dashes, and 9 hyphens = 3 em dashes).
When a heterogeneous sequence must be used,
the em dashes come first, followed by the en
dashes, and as few en dashes as possible are
used (so, 7 hyphens = 2 em dashes an 1 en
dash).

```````````````````````````````` example
one-
two--
three---
four----
five-----
six------
seven-------
eight--------
nine---------
thirteen-------------.
.
<p>one-
two–
three—
four––
five—–
six——
seven—––
eight––––
nine———
thirteen———––.</p>
````````````````````````````````

Hyphens can be escaped:

```````````````````````````````` example
Escaped hyphens: \-- \-\-\-.
.
<p>Escaped hyphens: -- ---.</p>
````````````````````````````````

Three periods form an ellipsis:

```````````````````````````````` example
Ellipses...and...and....
.
<p>Ellipses…and…and….</p>
````````````````````````````````

Periods can be escaped if ellipsis-formation
is not wanted:

```````````````````````````````` example
No ellipses\.\.\.
.
<p>No ellipses...</p>
````````````````````````````````