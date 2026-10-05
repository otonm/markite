# Markite Stress Test Document

> A deliberately messy, long Markdown file for exercising the editor, the gutter,
> horizontal scrolling, wrapping, the minimap, and editor ↔ preview scroll sync.

This paragraph is intentionally very long so that, with **Wrap Text** disabled, it pushes the editor into horizontal scrolling and lets you verify that the line-number gutter stays pinned to the left edge while the text slides underneath it — keep reading all the way to the end of this sentence because it really does go on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on and on for a while.

---

## Table of Contents

1. [Headings](#headings)
2. [Text formatting](#text-formatting)
3. [Lists](#lists)
4. [Code](#code)
5. [Tables](#tables)
6. [Quotes](#quotes)
7. [Links & images](#links--images)
8. [Task lists](#task-lists)
9. [Footnotes](#footnotes)
10. [Edge cases](#edge-cases)

---

## Headings

# H1 heading
## H2 heading
### H3 heading
#### H4 heading
##### H5 heading
###### H6 heading

Alt H1
======

Alt H2
------

## Text formatting

Normal, *italic*, **bold**, ***bold italic***, ~~strikethrough~~, `inline code`,
and a mix: **bold with `code` and *nested italic* inside**.

Subscript/superscript are not standard: H2O, x^2 (render as literal unless extended).

Escapes: \*not italic\*, \`not code\`, \# not a heading, 1\. not a list.

Unicode: café, naïve, 日本語, Ελληνικά, Кириллица, 😀🚀🔥, mathematical ∑∫∂√π≈≠≤≥.

## Lists

### Unordered

- First
- Second
  - Nested second-level
    - Nested third-level
      - Nested fourth-level with a long line that keeps going so wrapping and horizontal scroll both get a workout here as well
  - Back to second-level
- Third

### Ordered

1. One
2. Two
   1. Two-point-one
   2. Two-point-two
      1. Deeply nested ordered item
3. Three
42. Numbering that does not start at one (Markdown renumbers)

### Mixed

1. Step one
   - sub bullet
   - another sub bullet with `code`
2. Step two

      A loose paragraph belonging to step two, indented.

3. Step three

## Code

Inline: `let x = 42;` and `rustc --edition 2021 main.rs`.

```rust
// A Rust block with a long line to test horizontal scroll inside code.
fn main() {
    let very_long_variable_name_that_forces_horizontal_scrolling = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    println!("{}", very_long_variable_name_that_forces_horizontal_scrolling);
    for i in 0..10 {
        println!("line {i}");
    }
}
```

```python
def fib(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a
```

```json
{
  "name": "markite",
  "nested": { "a": [1, 2, 3], "b": { "deep": true } },
  "long": "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
}
```

```
Plain fenced block, no language.
~~~ tildes inside a backtick fence ~~~
```

~~~
Tilde-fenced block.
``` backticks inside a tilde fence ```
~~~

## Tables

| Left | Center | Right | Notes |
|:-----|:------:|------:|:------|
| a | b | c | short |
| longer cell content | centered | 3.14159 | a cell with a deliberately long value to test table overflow and horizontal scroll behaviour |
| `code` | **bold** | *italic* | ~~strike~~ |
| 1 | 2 | 3 | 4 |

| Single column |
|---------------|
| row 1 |
| row 2 |

## Quotes

> Level 1 quote.
>
> > Level 2 nested quote with a fairly long line so you can watch wrapping behave inside a blockquote context without surprises.
> >
> > > Level 3 nested quote.
>
> Back to level 1 with a list:
> - item a
> - item b

## Links & images

[Inline link](https://example.com), [link with title](https://example.com "Example"),
<https://autolink.example.com>, and a reference link [like this][ref].

[ref]: https://example.com/reference

![Alt text for an image](https://example.com/image.png "Image title")

Broken image (should show alt): ![missing](does-not-exist.png)

## Task lists

- [x] Completed task
- [ ] Open task
- [x] Another done item
  - [ ] Nested open subtask
  - [x] Nested done subtask

## Footnotes

Here is a statement needing a citation.[^1] And another.[^long]

[^1]: The first footnote.
[^long]: A longer footnote with **formatting**, `code`, and a [link](https://example.com).

## Horizontal rules

Three ways:

---

***

___

## Edge cases

Hard line break (two trailing spaces):  
this is on a new line.

Backslash break:\
also a new line.

An empty code span: `` ` `` (a literal backtick).

HTML passthrough:

<div style="padding:4px;border:1px solid gray">
  Raw <strong>HTML</strong> block with <em>inline</em> tags.
</div>

Entities: &copy; &amp; &lt; &gt; &hearts; &#8734;

A really wide no-space string to guarantee horizontal scrolling even with short words:
ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_ABCDEFGHIJKLMNOPQRSTUVWXYZ

Trailing whitespace and tabs:
	indented with a tab
    indented with four spaces (this becomes a code block)

## Long filler section for scroll testing

Paragraph 1 — lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.

Paragraph 2 — ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.

Paragraph 3 — duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.

Paragraph 4 — excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

Paragraph 5 — sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium.

Paragraph 6 — totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo.

Paragraph 7 — nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit.

Paragraph 8 — neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit.

Paragraph 9 — quis autem vel eum iure reprehenderit qui in ea voluptate velit esse quam nihil molestiae consequatur.

Paragraph 10 — at vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum deleniti atque corrupti.

### Numbered filler (1–30)

1. Line one of the long numbered filler block.
2. Line two.
3. Line three.
4. Line four.
5. Line five.
6. Line six.
7. Line seven.
8. Line eight.
9. Line nine.
10. Line ten — crossing into double digits to test gutter width changes.
11. Line eleven.
12. Line twelve.
13. Line thirteen.
14. Line fourteen.
15. Line fifteen.
16. Line sixteen.
17. Line seventeen.
18. Line eighteen.
19. Line nineteen.
20. Line twenty.
21. Line twenty-one.
22. Line twenty-two.
23. Line twenty-three.
24. Line twenty-four.
25. Line twenty-five.
26. Line twenty-six.
27. Line twenty-seven.
28. Line twenty-eight.
29. Line twenty-nine.
30. Line thirty — end of filler.

The end. If the gutter, minimap, wrap toggle, and scroll sync all held up through
this document, they work.
