import type { DataMessagePartProps } from "@assistant-ui/react";
import { createContext, useContext, useMemo, useState } from "react";
import { isRecord, readString, useUi } from "@zeokeel/ui-kit";

export const QuestionAnswerContext = createContext<(questionId: string, answers: unknown) => void>(
  () => {},
);

type Option = { label: string; description?: string };
type Question = { id: string; question: string; header?: string; options: Option[]; multi_select?: boolean };

function readQuestions(value: unknown): Question[] {
  if (!Array.isArray(value)) {
    return [];
  }
  const out: Question[] = [];
  for (const item of value) {
    if (!isRecord(item)) {
      continue;
    }
    const id = readString(item, "id");
    const question = readString(item, "question");
    if (id === null || question === null) {
      continue;
    }
    const options: Option[] = Array.isArray(item["options"])
      ? item["options"].flatMap((option) => {
          if (!isRecord(option)) {
            return [];
          }
          const label = readString(option, "label");
          if (label === null) {
            return [];
          }
          const description = readString(option, "description");
          return [description === null ? { label } : { label, description }];
        })
      : [];
    const header = readString(item, "header");
    out.push({ id, question, ...(header === null ? {} : { header }), options, multi_select: item["multi_select"] === true });
  }
  return out;
}

export function QuestionDataPart(props: DataMessagePartProps) {
  const { t } = useUi();
  const onAnswer = useContext(QuestionAnswerContext);
  const data = props.data;
  if (!isRecord(data)) {
    return null;
  }
  const id = readString(data, "id");
  const answered = data["answers"] !== null && data["answers"] !== undefined;
  const questions = useMemo(() => readQuestions(data["questions"]), [data]);
  const [selected, setSelected] = useState<Record<string, string[]>>({});
  const [custom, setCustom] = useState<Record<string, string>>({});
  if (id === null || questions.length === 0) {
    return null;
  }
  if (answered) {
    return (
      <div role="status" className="mt-3 rounded-document bg-muted px-3 py-2.5 text-[13px] text-muted-foreground">
        {t("questionAnswered")}
      </div>
    );
  }
  return (
    <div role="group" aria-label={t("questionTitle")} className="mt-3 rounded-document bg-muted px-3 py-2.5">
      <strong className="text-[13px] font-medium text-foreground">{t("questionTitle")}</strong>
      {questions.map((question) => (
        <fieldset key={question.id} className="mt-2 border-0 p-0">
          <legend className="text-[13px] text-foreground">{question.header ?? question.question}</legend>
          {question.options.length > 0 ? (
            <div className="mt-1.5 flex flex-col gap-1">
              {question.options.map((option) => (
                <label key={option.label} className="flex items-start gap-2 text-[13px] text-foreground">
                  <input
                    type={question.multi_select === true ? "checkbox" : "radio"}
                    name={`${id}-${question.id}`}
                    value={option.label}
                    checked={(selected[question.id] ?? []).includes(option.label)}
                    onChange={(event) => {
                      setSelected((current) => {
                        const existing = current[question.id] ?? [];
                        const next = question.multi_select === true
                          ? event.target.checked ? [...existing, option.label] : existing.filter((label) => label !== option.label)
                          : [option.label];
                        return { ...current, [question.id]: next };
                      });
                    }}
                  />
                  <span>
                    {option.label}
                    {option.description === undefined ? null : <span className="ms-1 text-[12px] text-muted-foreground">{option.description}</span>}
                  </span>
                </label>
              ))}
            </div>
          ) : (
            <input
              aria-label={question.question}
              className="mt-1.5 w-full rounded-control border border-border bg-background px-2 py-1 text-[13px]"
              value={custom[question.id] ?? ""}
              onChange={(event) => setCustom((current) => ({ ...current, [question.id]: event.target.value }))}
            />
          )}
        </fieldset>
      ))}
      <button
        type="button"
        className="mt-2.5 inline-flex items-center rounded-control bg-foreground px-3 py-1.5 text-[13px] font-medium text-background"
        onClick={() => {
          const answers = questions.map((question) => ({
            id: question.id,
            selected: selected[question.id] ?? [],
            custom: custom[question.id] ?? "",
          }));
          onAnswer(id, { answers });
        }}
      >
        {t("questionSubmit")}
      </button>
    </div>
  );
}
