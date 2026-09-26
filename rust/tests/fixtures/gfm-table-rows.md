# Table rows without a leading pipe

Body rows may start with characters that open other constructs; they stay rows unless that construct matches.

| Name | Meaning |
| ---- | ------- |
item | starts with the letter i
example | starts with the letter e
`code` | starts with a backtick
~~struck~~ | starts with a tilde
*emphasis* | starts with an asterisk
**strong** | starts with two asterisks
_emphasis_ | starts with an underscore
#hash | starts with a number sign
{braces} | starts with a brace
$x$ | starts with a dollar sign
plain | a control row

Option | Effect
--- | ---
import-map | not an import statement
export-ready | not an export statement
indexed | still a row
## Heading | a heading ends the table

After the heading, a paragraph.

> Quote | Cell
> --- | ---
> inside | a block quote
> `quoted` | with a code span

- Key | Value
  --- | ---
  entry | in a list item
  *nested* | with emphasis

Text after the tables.
