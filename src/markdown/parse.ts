import { fromMarkdown } from 'mdast-util-from-markdown';
import { gfm } from 'micromark-extension-gfm';
import { gfmFromMarkdown } from 'mdast-util-gfm';
import { frontmatter } from 'micromark-extension-frontmatter';
import { frontmatterFromMarkdown } from 'mdast-util-frontmatter';
import { math } from 'micromark-extension-math';
import { mathFromMarkdown } from 'mdast-util-math';
import { directive } from 'micromark-extension-directive';
import { directiveFromMarkdown } from 'mdast-util-directive';
import { mdxjs } from 'micromark-extension-mdxjs';
import { mdxFromMarkdown } from 'mdast-util-mdx';
import type { Extension } from 'micromark-util-types';
import type { Root, Nodes } from 'mdast';

export interface ParseOptions {
  /** Parse as MDX (JSX, ESM, expressions); used for .mdx files. */
  mdx?: boolean;
  /** Treat `$x$` as math. Off by default because prices like "$5 and $10" would become math. */
  mathSingleDollar?: boolean;
}

// Only block directives (:::note / ::leaf); text directives would turn prose like "foo:bar" into syntax.
const blockDirectives: Extension = { flow: directive().flow };

export function parseMarkdown(text: string, opts: ParseOptions = {}): Root {
  const extensions: Extension[] = [gfm(), frontmatter(['yaml', 'toml']), math({ singleDollarTextMath: !!opts.mathSingleDollar }), blockDirectives];
  const mdastExtensions = [gfmFromMarkdown(), frontmatterFromMarkdown(['yaml', 'toml']), mathFromMarkdown(), directiveFromMarkdown()];
  if (opts.mdx) {
    extensions.push(mdxjs());
    mdastExtensions.push(mdxFromMarkdown());
  }
  return fromMarkdown(text, { extensions, mdastExtensions });
}

export function startOf(node: Nodes): number {
  const o = node.position?.start.offset;
  if (o === undefined) throw new Error(`node ${node.type} has no position`);
  return o;
}

export function endOf(node: Nodes): number {
  const o = node.position?.end.offset;
  if (o === undefined) throw new Error(`node ${node.type} has no position`);
  return o;
}
