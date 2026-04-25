import re
with open('src/fixer/mod.rs', 'r') as f:
    fixer_content = f.read()

fixer_content = re.sub(
    r'Expression::Literal\(Literal::String\(s\), \.\.\)\s*=>\s*\{\s*if &\*\*s == "\\n"\s*\{\s*1\s*\}\s*else\s*\{\s*0\s*\}\s*\}',
    r'Expression::Literal(Literal::String(s), ..) if &**s == "\\n" => 1,\n            Expression::Literal(Literal::String(_), ..) => 0,',
    fixer_content
)

with open('src/fixer/mod.rs', 'w') as f:
    f.write(fixer_content)
