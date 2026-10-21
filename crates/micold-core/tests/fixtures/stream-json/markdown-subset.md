# Heading one

Setext heading two
------------------

### Heading three
#### Heading four
##### Heading five
###### Heading six

A paragraph with **bold**, *italic*, ~~struck~~ and `inline code`, a soft
break, and a hard break here\
then the next line. Links: [the web](https://example.com/web), [mail](mailto:team@example.com),
<https://example.com/auto>, a bare https://example.com/bare address, [script](javascript:alert)
a [relative](docs/readme) one and [a file](file:///etc/hosts).

An image ![a teapot](https://example.com/teapot.png) is never fetched.

<div class="note">
raw HTML block
</div>

Inline <span>raw html</span> stays literal.

A [later reference][later] is defined only in a later block.

[later]: https://example.com/later

- bullet one
- bullet two
  - nested bullet
    - nested deeper
- bullet three

3. third
4. fourth

1) loose one

2) loose two

- [x] done task
- [ ] open task

> A quote with **bold**
> > and a nested quote

---

***

| Left | Centre | Right | Plain |
|:-----|:------:|------:|-------|
| a    | b      | c     | d     |
| long cell text | `code` | [l](https://example.com/cell) | |

| no | separator |
| just | pipes |

An unmatched **bold opener stays literal.

    indented code
      keeps its indent

A paragraph written
with CRLF line endings

- and a
- CRLF list

```rust
fn main() {
    println!("hi");
}
```

```teapot-lang
brew black
```

```
untagged
```

~~~ python {.numbered}
print("tilde")
~~~

```sh
echo "this fence is never closed"
still code
