import { fromMarkdown } from 'mdast-util-from-markdown';
import { gfm } from 'micromark-extension-gfm';
import { gfmFromMarkdown } from 'mdast-util-gfm';
import { frontmatter } from 'micromark-extension-frontmatter';
import { frontmatterFromMarkdown } from 'mdast-util-frontmatter';
import type { Root, Nodes } from 'mdast';

export function parseMarkdown(text: string): Root {
  return fromMarkdown(text, {
    extensions: [gfm(), frontmatter(['yaml', 'toml'])],
    mdastExtensions: [gfmFromMarkdown(), frontmatterFromMarkdown(['yaml', 'toml'])],
  });
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
