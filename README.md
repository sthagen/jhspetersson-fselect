# fselect
Find files with SQL-like queries

### Examples

Find images (full path and size):

    fselect path, size from /home/user where name = *.jpg or name = *.png

Find files (just names) with any content (size > 0):

    fselect name from /home/user/tmp where size gt 0

or put arguments into the quotes:

    fselect "name from /home/user/tmp where size > 0"
    
More complex query:

    fselect name from /tmp where (name = *.tmp and size = 0) or (name = *.cfg and size gt 1000000)
    
Use single quotes if you need to address files with spaces:

    fselect path from '/home/user/Misc stuff' where name != 'Some file'
    
Regular expressions supported:

    fselect name from /home/user where path ~= .*Rust.*
    
And even simple glob will suffice:

    fselect name from /home/user where path = *Rust*

### Columns and expression fields

* `path`
* `name`
* `size`

### Operators

* `=` or `eq`
* `!=` or `ne`
* `>` or `gt`
* `>=` or `gte`
* `<` or `lt`
* `<=` or `lte`
* `~=` or `regexp` or `rx`
