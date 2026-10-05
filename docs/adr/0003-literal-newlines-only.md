# Literal Newlines Only for Multi-line Delimitation

Dion treats only genuine newline characters (CRLF or LF) as multi-line comment delimiters. In-flight string characters matching `\n` are strictly treated as literal backslash and character 'n' rather than synthetic linebreaks. This eliminates shell-quoting ambiguity and adheres strictly to the KISS principle.
