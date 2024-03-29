markdown-mode Test Cases
========================

## 1. Lists

Unordered lists:

- This is a bullet point.
  - This is a sub bullet point.
- This is another bullet point.

Ordered lists:

1. This is an ordered list
2. With a second element.
44. And a forty-fourth element.
3. Remember, Markdown doesn't care which number you use.

List items with bold and italic:

> * This is a list item *in italics*, just a test.
> * *List item in italics.*
> * This is a list item **in bold**, just a test.
> * **List item in bold.**

Bold and italic phrases at the beginning of lines:

*not a list*
**also not a list**


## 2. Blockquotes


> this is a test
> of the blockquote mechanism


## 3. Two Inline Links on One Line


*Contributed by Ryan Barrett on 2007-07-02*

thanks! the new version 1.4 works great. very cool.

i did notice a minor bug. if there are two inline links in the same line, e.g.
[foo](bar) baz [foo](bar), it colors the text between the links (baz) as well.

i'm guessing this is because the inline link regexp is greedy. if you switch
it to non-greedy - if elisp can do that - or to something like '\[[^]]+\]' if
not, that might do the trick.


## 4. Empty Inline Links

[]()
[](asdf)
[asdf]()


## 5. Bold and Italics on the Same Line

*Contributed by Conal Elliott on 2007-08-27*

By the way, here are two syntax-highlighting glitches I've seen so far:

**foo and doo** or *ziddle zop*
Oh -- what's happening here?


## 6. Reverse Order Italic then Bold

Why doesn't the bold part get picked up?
*ziddle zop* or **foo and doo**


## 7. Two Inline Links in One Line

*Contributed by Alastair Rankine on 2007-09-10*

Thanks for this, I have been waiting for an emacs markdown mode for ages!

One minor quibble is in the highlighting of inline links. As currently
(1.4) implemented the presence of a close paren on the same line as an
inline link will highlight the link and the subsequent text:

   [blah](this bit will be highlighted) and so (will this bit)

I think this is because of greedy regexes. Here is a simple patch to address.


## 8. Reference-Style Link with a Space

Here's a [link] [1] with a space between the components.

 [1]: http://daringfireball.net/projects/markdown/syntax


## 9. Inline Code

Single `backtick code` fragments as well as ``double`backtick`` fragments are
supported.


## 10. Single Underscore and Asterisk

This is a _ single underscore and a * single asterisk.
They should not trigger font locking.


## 11. Double Underscores and Asterisks

Neither should a double **asterisk and double__ underscore!

## 12. List-Like Strings in Preformatted Text

To make lists look nice, you can wrap items with hanging indents:

    *   Lorem ipsum dolor sit amet, consectetuer adipiscing elit.
        Aliquam hendrerit mi posuere lectus. Vestibulum enim wisi,
        viverra nec, fringilla in, laoreet vitae, risus.
    *   Donec sit amet nisl. Aliquam semper ipsum sit amet velit.
        Suspendisse id sem consectetuer libero luctus adipiscing.

## 13. Multi-Line Italic and Bold

bold **phrase spanning
two lines** asdf **same line** test

italic *phrase spanning
two lines* italic *same line* test


## 14. Reference Links

This document was created in GNU Emacs using [markdown-mode][].  It provides
syntax highlighting for [Markdown][] documents which allows reference-style
links like these.  The reference definitions can even contain alternate
text such as [this one][id].

 [markdown-mode]: http://jrblevin.freeshell.org/software/markdown-mode
 [Markdown]:      http://daringfireball.net/projects/markdown "Markdown Homepage"
 [id]:            http://reference-link.com/with/alt "Alternate Text"


## 15. Escaping

ab \*literal asterisks\* asdf *inside \* literal asterisks* asd lkj

ab \**this should be italic\** yz  
ab \**this should be italic*\* yz  
ab *\*this should be italic\** yz  
ab *\*this should be italic*\* yz


## 16. Single Letter and Word on Same Line

a single **a** bold letter and bold word **test** asdf  
a single _a_ italic letter and italic word _test_ asdf  
a single __a__ bold letter and bold word __test__ asdf  
a single *a* italic letter italic word *test* asdf


## 17. Hanging Indents

*   An exclamation mark: `!`;
*   followed by a set of square brackets, containing the `alt`
    attribute text for the image;


## 18. Links in Preformatted Text

Here's an example of reference links in action:

    This document was created in GNU Emacs using [markdown-mode][].  It
    provides syntax highlighting for [Markdown][] documents which allows
    reference-style links like these.  The reference definitions can even
    contain alternate text such as [this one][id].
    
     [markdown-mode]: http://jrblevin.freeshell.org/software/markdown-mode
     [Markdown]:      http://daringfireball.net/projects/markdown
     [id]:            http://reference-link.com/with/alt "Alternate Text"


## 19. Slash-Star in Preformatted Text

    svn commit -m "Imported RCS project"
    svn add image.png binaries/*
    svn commit -m "Imported additional project files"

This text gets counted as part of the block quote too.  But it can be ended
by a star-slash: */ It is as if `markdown-mode` is using C++ style comments.

Fixed by commit d81138d.


## 20. Underscores in Code Blocks

*Contributed by shindo on 2008-01-20*

    get_something_by_name()


## 21. Escaped Characters

Escaped backticks: \`not code\`

Escaped underscores: don't\_italicize\_this

Escaped asterisks: \*also not italic\*

Escaped hash marks:  
\# This is not a heading


## 22. Adjacent Wiki Links

[[Two]] [[WikiLinks]] [[InARow]]


## 23. SmartyPants

This is a test of "SmartyPants," a progrm written by John Gruber for
generating typographically correct HTML entities--ones such as
the em-dash and ellipsis...


## 24. Horizontal Rules

* * * * *

- - - --- - - -

* *** * * *** * * *


## 25. Asterisks and Underscores Across Blocks

Asterisks *should

not match across* block boundaries.

Underscores _should

not match across_ block boundaries.

Double Asterisks **should

not match across** block boundaries.

Double underscores __should

not match across__ block boundaries.

But, *this should still match*.

So should _this_.

Addressed by commit d81138d.


## 26. Underscores Within Words

Code fragments `can_have_underscores_inside_like` this.


## 27. Code Blocks Spanning Lines

Markdown allows `code
fragments` to span across lines.

Let's make sure that ``double`
backtick`code fragments`` work
this way too.

However, they should `not

match` across blocks.

Here's another `valid` one.


## 28. Pre Blocks and Nested Lists

   - List level 1 item 1
   - List level 1 item 2

     This is a second paragraph, part of item 2.

         This nested pre block should match

             So should this

     Now we move back to the list.

   - List level 1 item 3

       - List level 2 item 1

             Nested pre block

       - List level 2 item 2

           - List level 3 item 1

                 Nested pre block


## 29. Colon After Wiki Link

[[AnotherLink]]:note


## 30. Incorrect Parsing of List Paragraph

*Contributed by Luciano Gerber <garopaba_uk@yahoo.co.uk> on April 4, 2012.*

It is interesting to see what happens when one queries
`social upheaval` and `protopalatial era`.

* `social upheaval`: the following queries have been tried:

    social upheaval subClassOf

The preceding text should not be matched as a preformatted block.


## 31. Footnotes

This is a footnote marker,[^1] but this is not.[^]

[^1]: And the definition of the footnote is here.

    [^2]: but this is preformatted text.


## 32. Carat as Implicit Reference Link

This is a valid markdown link: [^][]

[^]: http://jblevins.org/ "And this is a valid reference definition!"


## 33. Email Address Inside Bold and Italic Spans

*Lorem <ipsum@dolor.sit> amet.*

**Lorem <ipsum@dolor.sit> amet.**


## 34. URL Inside Bold and Italic Spans

*markdown-mode homepage http://jblevins.org/projects/markdown-mode/*

**markdown-mode homepage http://jblevins.org/projects/markdown-mode/**

## 35. Complex Preformatted Code Block

    class Employee < ActiveRecord::Base
      attr_accessible :github_username, :name
    
      def self.syncify!
        begin
          employee_sync = GitOrganized::GithubEmployeeSync.new(
            GitOrganized::Organization.new(
              GitOrganized.config.default_org,
              ["employees-pull", "employees-push"]
            ),
            Sources.new
          )
    
          employee_sync.sync
        rescue Github::Error::ServiceError => e
          retry
        end
      end
    
      after_save do |employee| Employee.syncify! end
      after_destroy do |employee| Employee.syncify! end
    end

## 36. Fenced Code Blocks

Fenced code blocks begin with three or more tildes and end with a line
with at least as many tildes.

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
program hello
  implicit none
  print '(a)', 'Hello, world!'
end program hello
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

They may also have a `lang` attribute.

~~~~~~~~~~~~~{: lang=fortran }
program hello
  implicit none
  print '(a)', 'Hello, world!'
end program hello
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

In Kramdown, one specifies the language identifier as follows:

~~~ fortran
program hello
  implicit none
  print '(a)', 'Hello, world!'
end program hello
~~~

or with a Maruku-style inline attribute list

~~~
program hello
  implicit none
  print '(a)', 'Hello, world!'
end program hello
~~~
{: .language-fortran}