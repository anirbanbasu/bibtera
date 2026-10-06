+++
title = {{ title | json_encode }}
template = "page.html"

[extra]
citation_key = {{ key | json_encode }}
entry_type = {{ entry_type | json_encode }}
authors = {{ authors | json_encode }}
year = {{ year | default(value="") | json_encode }}
+++

**Authors**: {{ authors | join(sep=", ") }}

{% raw %}{{ <cite key="{% endraw %}{{ key }}{% raw %}" /> }}{% endraw %}
