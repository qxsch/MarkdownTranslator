# Tricky document

This paragraph is soft wrapped over several lines so that the translator has to
re-wrap the translated text to a similar width without breaking inline `code
spans` or [link targets](https://example.com/a/b "with a title") in the middle of
the paragraph.

> A quoted paragraph that is also wrapped over
> multiple lines, with **bold text** and a file named
> settings.local.json inside the quote.
>
> ```js
> // comment inside a quoted code block
> const x = 1;
> ```

- A list item with a nested paragraph

  and a second paragraph that
  continues on another line.

  ```yaml
  # yaml comment in a list item
  key: value # trailing comment
  url: "http://example.com/#anchor"
  ```

A shortcut reference to [Contoso] and a collapsed one to [Fabrikam][].

Snake_case_identifier, camelCaseName and PascalCaseType must survive, as must Get-AzContext and v1.2.3.

<table>
  <tr><th>Name</th><th>Meaning</th></tr>
  <tr><td>Alpha</td><td>The first letter of the alphabet</td></tr>
</table>

<details>
<summary>Click to expand the details</summary>

Hidden content with a [link](#tricky-document) back to the top.

</details>

```csharp
/// <summary>
/// Gets the value of the <see cref="Foo"/> property.
/// </summary>
/// <param name="id">The identifier of the item.</param>
public int Get(int id) => id; // returns the id
```

```java
/**
 * Computes the total price.
 *
 * @param items the items in the cart
 * @return the total price including tax
 */
int total(List<Item> items) { return 0; }
```

```sql
-- Select the active users
SELECT * FROM users WHERE name = 'O''Brien -- not a comment';
```

[Contoso]: https://contoso.com
[Fabrikam]: https://fabrikam.com
