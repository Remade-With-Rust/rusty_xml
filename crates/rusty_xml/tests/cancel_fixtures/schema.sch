<schema xmlns="http://purl.oclc.org/dsdl/schematron">
<pattern><rule context="item"><assert test="@id">item needs an id</assert><report test="@id = 0">zero id</report></rule></pattern>
</schema>
