# {{ title }}

<!-- citation: {{ key }} -->

**Authors**: {{ authors | join(sep=", ") }}

**Year**: {{ year | default(value="Unknown") }}

**Type**: {{ entry_type }}

{% if fields %}
**Fields**:

{% for key, value in fields %}
- **{{ key }}**: {{ value }}
{% endfor %}
{% endif %}
