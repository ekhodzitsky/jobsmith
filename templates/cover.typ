// Professional Cover Letter — Russian business format
#set page("a4", margin: (x: 2.2cm, y: 2.2cm))
#set text(font: ("Liberation Sans", "Helvetica Neue", "Arial"), size: 11pt, lang: "ru")
#set par(justify: true, leading: 0.58em, first-line-indent: 0em)

// Date
#align(right)[{{DATE}}]
#v(1.2em)

// Sender block
#align(left)[
  *{{CANDIDATE_NAME}}*
  #linebreak()
  {{CANDIDATE_CITY}}
  #linebreak()
  {{CANDIDATE_PHONE}}
  #linebreak()
  {{CANDIDATE_EMAIL}}
]
#v(1.2em)

// Recipient block
#align(right)[
  {{COMPANY}}
  #linebreak()
  {{ROLE}}
]
#v(1.5em)

// Subject line
#align(center)[
  #text(weight: "bold")[Резюме на позицию «{{ROLE}}»]
]
#v(1em)

// Body text
{{CONTENT}}
#v(1.8em)

// Signature
#align(left)[
  С уважением,
  #v(1.2em)
  *{{CANDIDATE_NAME}}*
]
