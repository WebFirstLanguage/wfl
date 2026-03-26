import re

with open("src/stdlib/helpers.rs", "r") as f:
    content = f.read()

replacements = [
    ("expect_text", "Text", "Arc<str>", "text"),
    ("expect_list", "List", "Rc<RefCell<Vec<Value>>>", "a list"),
    ("expect_date", "Date", "Rc<chrono::NaiveDate>", "a Date"),
    ("expect_time", "Time", "Rc<chrono::NaiveTime>", "a Time"),
    ("expect_datetime", "DateTime", "Rc<chrono::NaiveDateTime>", "a DateTime"),
]

for func_name, variant, ret_type, expected_name in replacements:
    # Pattern to match the docstrings and the function body
    pattern = r"(///[^\n]*\n)*pub fn " + func_name + r"\(value: &Value\) -> Result<[^>]+, RuntimeError> \{.*?\n\}"
    match = re.search(pattern, content, re.DOTALL)
    if match:
        full_match = match.group(0)

        # Extract the docstrings
        docs = []
        for line in full_match.split("\n"):
            if line.strip().startswith("///"):
                docs.append(line)
            elif line.strip().startswith("pub fn"):
                break

        doc_str = "\n".join(docs)
        if doc_str:
            # rust requires the meta block to be part of the macro syntax for attributes
            doc_str += "\n"

        macro_call = f"generate_expect!(\n{doc_str}    {func_name}, {variant}, {ret_type}, \"{expected_name}\"\n);"

        content = content[:match.start()] + macro_call + content[match.end():]

with open("src/stdlib/helpers.rs", "w") as f:
    f.write(content)
