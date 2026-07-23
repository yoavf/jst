# Phi-4 prompting research for `jst`

## Executive summary

Microsoft does not publish a standalone, Phi-4-specific “prompting guide.” The authoritative guidance is split between the [Phi-4 model card](https://huggingface.co/microsoft/phi-4), the [shipped tokenizer/chat template](https://huggingface.co/microsoft/phi-4/blob/main/tokenizer_config.json), and the [Phi-4 Technical Report](https://arxiv.org/html/2412.08905). Together they suggest a clear direction:

1. Use Phi-4’s native ChatML `system`/`user`/`assistant` structure exactly.
2. Optimize for a focused single-turn task, not a long conversational prompt.
3. Treat strict JSON and detailed instruction following as known weak points; constrain, exemplify, validate, and retry.
4. Test compact few-shot examples because Microsoft found that examples helped answer-format adherence, but do not assume more context is automatically better.
5. Prefer a short, ordered contract and a silent verification checklist over asking for visible chain-of-thought.

The last three points are experimental recommendations inferred from Microsoft’s evidence, not claims that Microsoft has published one universally optimal prompt.

## What Microsoft documents

### Use the native chat format

Phi-4 is “best suited” to chat-formatted prompts and has a 16K-token context window. Microsoft’s model card demonstrates a `system` message followed by a `user` message and an open `assistant` turn ([model card, Input Formats](https://huggingface.co/microsoft/phi-4#input-formats)).

The exact shipped template is:

```text
<|im_start|>system<|im_sep|>...<|im_end|>
<|im_start|>user<|im_sep|>...<|im_end|>
<|im_start|>assistant<|im_sep|>
```

Use the provider’s chat-completions interface or `tokenizer.apply_chat_template(..., add_generation_prompt=True)` rather than constructing tokens by hand ([model card usage](https://huggingface.co/microsoft/phi-4#with-transformers), [tokenizer template](https://huggingface.co/microsoft/phi-4/blob/main/tokenizer_config.json)). The tokenizer template handles only `system`, `user`, and `assistant`; applications using the raw template should not rely on extra roles such as `developer` or `tool` being preserved ([tokenizer template](https://huggingface.co/microsoft/phi-4/blob/main/tokenizer_config.json)).

The technical report confirms that post-training used standard ChatML and shows both single- and multi-turn structure ([technical report, §4](https://arxiv.org/html/2412.08905)).

### Strict instruction following and formatting are known weaknesses

Microsoft identifies IFEval as a real weakness and says Phi-4 has trouble strictly following instructions ([technical report, §6](https://arxiv.org/html/2412.08905)). Its weaknesses section is more specific: detailed formatting requirements, predefined bullet structures, and precise stylistic constraints can be violated ([technical report, §8](https://arxiv.org/html/2412.08905)).

This applies directly to `jst`: a large prose rule set plus a strict nested JSON schema asks Phi-4 to do something Microsoft says is comparatively weak. API-level JSON mode is still worth using, but it does not remove the need to parse and semantically validate the result.

Microsoft also observed that tested models, including Phi-4, often failed a required final-answer format after a long reasoning trace ([technical report, Appendix C](https://arxiv.org/html/2412.08905)). For command translation, do not request visible step-by-step reasoning. Ask for the final JSON only and, if useful, tell the model to verify constraints silently before responding.

### Few-shot examples are a plausible lever, with qualifications

During pretraining evaluation, Microsoft used 1, 3, 4, or 8 examples for several tasks “to help the model adhere to the answer format” ([technical report, §3](https://arxiv.org/html/2412.08905)). That evidence is for the pretrained checkpoint’s benchmark setup, not a controlled study of the final chat model, so it supports testing few-shot prompting rather than assuming it will win.

For `jst`, examples should teach semantic decisions as well as JSON syntax. A small set should cover the failure classes of interest:

- target-OS portability;
- preserving explicit constraints such as “same filesystem” or “do not overwrite”;
- treating text literally rather than expanding substitutions;
- flagging commands that transmit likely credentials;
- exact, minimal JSON with correct effect flags.

Compare examples embedded compactly in the system message with examples represented as prior `user`/`assistant` turns. Phi-4 supports multi-turn ChatML, but Microsoft says it was fine-tuned to maximize single-turn performance ([technical report, §8](https://arxiv.org/html/2412.08905)), so the result should be measured rather than presumed.

### More prompt is not necessarily better

Phi-4 supports 16K context, but Microsoft’s HELMET results are mixed when moving from 8K to 16K: some tasks improve while RAG and re-ranking decline ([technical report, §3.2, Table 6](https://arxiv.org/html/2412.08905)). This is not proof that a short `jst` prompt is always superior, but it argues against adding context without measuring its marginal value.

The model can also produce long, elaborate answers to simple problems because its training contains many chain-of-thought examples, and it is optimized more for single-turn queries than extended chat ([technical report, §8](https://arxiv.org/html/2412.08905)). Keep the task focused and cap output length tightly enough to fit the schema.

## Recommended prompt shape to test first

Use the system message for stable policy and the user message for per-request data. Put the non-negotiable task and output contract ahead of explanatory taxonomy:

```text
system:
You translate one natural-language request into one shell command.

Priority requirements:
1. The command must exactly satisfy every explicit user constraint.
2. It must be valid for the supplied OS and shell.
3. Return only one JSON object matching OUTPUT_SCHEMA.
4. If no command can satisfy the request, use "# unable to translate".

Before responding, silently verify:
- command semantics match the request;
- OS and shell compatibility;
- quoting preserves literal text;
- no requested safety or non-overwrite constraint was dropped;
- every required JSON field is present and correctly typed.

OUTPUT_SCHEMA:
<compact schema or canonical JSON example>

<optional compact examples>

user:
{"request":"...","os":"macos","shell":"/bin/zsh"}
```

This shape is a research candidate, not an official Microsoft template. Its rationale is to align with native roles, reduce competing prose, avoid visible reasoning, and make exact constraints salient.

## Autoresearch experiment axes

Change one dimension at a time, run each candidate repeatedly, and retain a fixed holdout set so prompt search does not overfit issues 30–34.

| Axis | Variants worth testing |
|---|---|
| Role structure | canonical `system` + structured `user`; all-in-system; compact examples as ChatML turns |
| Length | minimal contract; current full taxonomy; minimal contract plus only definitions that affect scoring |
| Ordering | task → priorities → schema → examples; schema first; environment immediately before request |
| User context | plain request; labeled text blocks; compact JSON `{request, os, shell}` |
| Examples | 0, 1, 3, 5; generic examples; failure-class-targeted examples; positive plus corrected-negative examples |
| Verification | none; one silent checklist; explicit “draft then verify internally, output only final JSON” |
| Schema guidance | prose field list; one canonical JSON object; concise JSON Schema if the serving API enforces it |
| Output budget | tight schema-sized cap; current cap; oversized control |
| Recovery | no retry; parse retry; targeted retry containing only the detected semantic defect |

Use deterministic parsing and executable or rule-based graders wherever possible. Microsoft’s own synthetic code pipeline used execution loops and tests for validation, and its training pipeline used iterative critique/revision workflows ([technical report, §2.2](https://arxiv.org/html/2412.08905)). This does not prove that self-revision at inference will help, but it makes a targeted repair pass a reasonable experiment.

Suggested optimization order:

1. Establish native-role, zero-shot, short-prompt baseline.
2. Add one canonical JSON example.
3. Add three targeted semantic examples.
4. Try the silent checklist.
5. Remove prompt clauses that do not improve held-out scores.
6. Add a targeted repair request only for machine-detectable failures.

Track at least: exact JSON validity, schema validity, command semantic correctness, OS/shell correctness, constraint preservation, safety/effect-label correctness, latency, input/output tokens, and consistency across repeated samples.

## Caveats relevant to command generation

- Phi-4’s model card says most code training is Python and recommends manual verification of API use for other packages and languages; shell output therefore needs its own execution/static checks ([model card, Responsible AI Considerations](https://huggingface.co/microsoft/phi-4#responsible-ai-considerations)).
- Microsoft calls out factual hallucination and says grounding can reduce but not eliminate it ([technical report, §8](https://arxiv.org/html/2412.08905)). Prompt text alone cannot guarantee correct platform-specific flags.
- The model is primarily trained in English; non-English requests can have worse performance ([model card](https://huggingface.co/microsoft/phi-4#model-summary)).
- Microsoft recommends application-level safeguards for consequential use and notes that model outputs may be inaccurate ([model card, Responsible AI Considerations](https://huggingface.co/microsoft/phi-4#responsible-ai-considerations)). For a shell executor, semantic validators and confirmation policy remain necessary even after prompt optimization.

## Primary sources

- Microsoft, [Phi-4 model card and usage](https://huggingface.co/microsoft/phi-4)
- Microsoft, [Phi-4 tokenizer configuration and chat template](https://huggingface.co/microsoft/phi-4/blob/main/tokenizer_config.json)
- Microsoft Research, [Phi-4 Technical Report](https://arxiv.org/html/2412.08905)
