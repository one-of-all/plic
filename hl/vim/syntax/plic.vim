if exists("b:current_syntax")
    finish
endif

" Keywords
syn keyword chKeyword   let if then else case of data struct try catch error for while lambda and or not class extends new loop break load del
syn keyword chKeyword   addContact removeContact newChat addMember removeMember deleteChat sendChat sendFileToChat inbox history downloads saveFile
syn keyword chKeyword   serverStart serverStop connect getPublicIP setExternalIP toFloat toInt typeof

" DataType
syn region  chString    start=+"+ end=+"+ skip=+\\"+    contains=@Spell
syn region  chString    start=+'+ end=+'+ skip=+\\'+    contains=@Spell

" f-string
syn region  chFString   start=+f"+ end=+"+ skip=+\\"+ contains=chFStringExpr
syn region  chFStringExpr start=+{+ end=+}+ contained contains=chKeyword,chString,chNumber,chType,chOperator,chComment

" Byte string
syn match   chByteString "#B\"[0-9A-Fa-f]*\""

" Numbers
syn match   chNumber    "\<\d\(\d\|[a-zA-Z_]\)*\>"
syn keyword chNumber    true false

" Type
syn keyword chType      Num Char String Bool Unit Uid ByteString Duration List Tuple Record Pid DateTime Json Maybe Either ChatMsg FileInfo FileTransfer FetchOptions FetchResult Map Set ClassInstance

" Operators
syn match   chOperator  "[-+*%/@&$|^~=<>?:]"
syn match   chOperator  "[{}()\[\];,.]"

" Comments
syn match   chComment   "#.*$"                          contains=@Spell
syn region  chComment   start="#-" end="-#"             contains=@Spell fold

" Markup
hi def link chComment       Comment
hi def link chTodo          Todo
hi def link chString        String
hi def link chFString       String
hi def link chByteString    String
hi def link chNumber        Number
hi def link chType          Type
hi def link chKeyword       Keyword
hi def link chOperator      Operator

let b:current_syntax = "plic"
