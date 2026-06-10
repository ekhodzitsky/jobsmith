// Professional CV — Russian conventions
#set page("a4", margin: (x: 1.6cm, y: 1.4cm))
#set text(font: ("Liberation Sans", "Helvetica Neue", "Arial"), size: 10.5pt, lang: "ru")
#set par(justify: true, leading: 0.52em)
#set heading(numbering: none)

// Header: Name
#align(center)[
  #text(size: 20pt, weight: "bold", tracking: 0.5pt)[{{NAME}}]
  #v(0.4em)
]

// Contact line
#align(center)[
  #text(size: 9.5pt, fill: rgb("#444444"))[{{CONTACT}}]
  #v(0.6em)
]

// Separator
#line(length: 100%, stroke: 0.6pt + rgb("#aaaaaa"))
#v(0.5em)

// Summary
== Профессиональное резюме
#v(0.2em)
{{SUMMARY}}
#v(0.6em)

// Experience
== Опыт работы
#v(0.2em)
{{EXPERIENCE}}
#v(0.6em)

// Skills
== Навыки
#v(0.2em)
{{SKILLS}}
#v(0.6em)

// Education
== Образование
#v(0.2em)
{{EDUCATION}}
#v(0.6em)

// Certifications
== Сертификаты и курсы
#v(0.2em)
{{CERTIFICATIONS}}
#v(0.6em)

// Languages
== Языки
#v(0.2em)
{{LANGUAGES}}
#v(0.6em)

// Publications
== Публикации и проекты
#v(0.2em)
{{PUBLICATIONS}}
