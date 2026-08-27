# rusty_xml API census

Generated from libxml2 **v2.15.3** (`oracle/src/include/libxml/*.h`).
Rust names are the mechanical snake_case of the C `XMLPUBFUN` identifier.

- Current (non-deprecated): **1011**
- Deprecated (porting guide only, not the facade): **369**

## Current

| header | libxml2 | rusty_xml |
|---|---|---|
| `c14n.h` | `xmlC14NDocSaveTo` | `xml_c14n_doc_save_to` |
| `c14n.h` | `xmlC14NDocDumpMemory` | `xml_c14n_doc_dump_memory` |
| `c14n.h` | `xmlC14NDocSave` | `xml_c14n_doc_save` |
| `c14n.h` | `xmlC14NExecute` | `xml_c14n_execute` |
| `catalog.h` | `xmlInitializeCatalog` | `xml_initialize_catalog` |
| `catalog.h` | `xmlLoadCatalog` | `xml_load_catalog` |
| `catalog.h` | `xmlLoadCatalogs` | `xml_load_catalogs` |
| `catalog.h` | `xmlCatalogCleanup` | `xml_catalog_cleanup` |
| `catalog.h` | `xmlCatalogDump` | `xml_catalog_dump` |
| `catalog.h` | `xmlCatalogDumpDoc` | `xml_catalog_dump_doc` |
| `catalog.h` | `xmlCatalogResolve` | `xml_catalog_resolve` |
| `catalog.h` | `xmlCatalogResolveSystem` | `xml_catalog_resolve_system` |
| `catalog.h` | `xmlCatalogResolvePublic` | `xml_catalog_resolve_public` |
| `catalog.h` | `xmlCatalogResolveURI` | `xml_catalog_resolve_uri` |
| `catalog.h` | `xmlCatalogAdd` | `xml_catalog_add` |
| `catalog.h` | `xmlCatalogFreeLocal` | `xml_catalog_free_local` |
| `catalog.h` | `xmlCatalogAddLocal` | `xml_catalog_add_local` |
| `catalog.h` | `xmlCatalogLocalResolve` | `xml_catalog_local_resolve` |
| `catalog.h` | `xmlCatalogLocalResolveURI` | `xml_catalog_local_resolve_uri` |
| `catalog.h` | `xmlCatalogSetDefaults` | `xml_catalog_set_defaults` |
| `catalog.h` | `xmlCatalogGetDefaults` | `xml_catalog_get_defaults` |
| `chvalid.h` | `xmlCharInRange` | `xml_char_in_range` |
| `debugXML.h` | `xmlDebugDumpString` | `xml_debug_dump_string` |
| `debugXML.h` | `xmlDebugDumpAttr` | `xml_debug_dump_attr` |
| `debugXML.h` | `xmlDebugDumpAttrList` | `xml_debug_dump_attr_list` |
| `debugXML.h` | `xmlDebugDumpOneNode` | `xml_debug_dump_one_node` |
| `debugXML.h` | `xmlDebugDumpNode` | `xml_debug_dump_node` |
| `debugXML.h` | `xmlDebugDumpNodeList` | `xml_debug_dump_node_list` |
| `debugXML.h` | `xmlDebugDumpDocumentHead` | `xml_debug_dump_document_head` |
| `debugXML.h` | `xmlDebugDumpDocument` | `xml_debug_dump_document` |
| `debugXML.h` | `xmlDebugDumpDTD` | `xml_debug_dump_dtd` |
| `debugXML.h` | `xmlDebugDumpEntities` | `xml_debug_dump_entities` |
| `debugXML.h` | `xmlDebugCheckDocument` | `xml_debug_check_document` |
| `dict.h` | `xmlDictCreate` | `xml_dict_create` |
| `dict.h` | `xmlDictSetLimit` | `xml_dict_set_limit` |
| `dict.h` | `xmlDictGetUsage` | `xml_dict_get_usage` |
| `dict.h` | `xmlDictCreateSub` | `xml_dict_create_sub` |
| `dict.h` | `xmlDictReference` | `xml_dict_reference` |
| `dict.h` | `xmlDictFree` | `xml_dict_free` |
| `dict.h` | `xmlDictLookup` | `xml_dict_lookup` |
| `dict.h` | `xmlDictExists` | `xml_dict_exists` |
| `dict.h` | `xmlDictQLookup` | `xml_dict_qlookup` |
| `dict.h` | `xmlDictOwns` | `xml_dict_owns` |
| `dict.h` | `xmlDictSize` | `xml_dict_size` |
| `encoding.h` | `xmlLookupCharEncodingHandler` | `xml_lookup_char_encoding_handler` |
| `encoding.h` | `xmlOpenCharEncodingHandler` | `xml_open_char_encoding_handler` |
| `encoding.h` | `xmlCreateCharEncodingHandler` | `xml_create_char_encoding_handler` |
| `encoding.h` | `xmlCharEncNewCustomHandler` | `xml_char_enc_new_custom_handler` |
| `encoding.h` | `xmlParseCharEncoding` | `xml_parse_char_encoding` |
| `encoding.h` | `xmlGetCharEncodingName` | `xml_get_char_encoding_name` |
| `encoding.h` | `xmlDetectCharEncoding` | `xml_detect_char_encoding` |
| `encoding.h` | `xmlCharEncOutFunc` | `xml_char_enc_out_func` |
| `encoding.h` | `xmlCharEncCloseFunc` | `xml_char_enc_close_func` |
| `encoding.h` | `xmlUTF8ToIsolat1` | `xml_utf8_to_isolat1` |
| `encoding.h` | `xmlIsolat1ToUTF8` | `xml_isolat1_to_utf8` |
| `entities.h` | `xmlNewEntity` | `xml_new_entity` |
| `entities.h` | `xmlFreeEntity` | `xml_free_entity` |
| `entities.h` | `xmlAddEntity` | `xml_add_entity` |
| `entities.h` | `xmlAddDocEntity` | `xml_add_doc_entity` |
| `entities.h` | `xmlAddDtdEntity` | `xml_add_dtd_entity` |
| `entities.h` | `xmlGetPredefinedEntity` | `xml_get_predefined_entity` |
| `entities.h` | `xmlGetDocEntity` | `xml_get_doc_entity` |
| `entities.h` | `xmlGetDtdEntity` | `xml_get_dtd_entity` |
| `entities.h` | `xmlGetParameterEntity` | `xml_get_parameter_entity` |
| `entities.h` | `xmlEncodeEntitiesReentrant` | `xml_encode_entities_reentrant` |
| `hash.h` | `xmlHashCreate` | `xml_hash_create` |
| `hash.h` | `xmlHashCreateDict` | `xml_hash_create_dict` |
| `hash.h` | `xmlHashFree` | `xml_hash_free` |
| `hash.h` | `xmlHashDefaultDeallocator` | `xml_hash_default_deallocator` |
| `hash.h` | `xmlHashAdd` | `xml_hash_add` |
| `hash.h` | `xmlHashAddEntry` | `xml_hash_add_entry` |
| `hash.h` | `xmlHashUpdateEntry` | `xml_hash_update_entry` |
| `hash.h` | `xmlHashAdd2` | `xml_hash_add2` |
| `hash.h` | `xmlHashAddEntry2` | `xml_hash_add_entry2` |
| `hash.h` | `xmlHashUpdateEntry2` | `xml_hash_update_entry2` |
| `hash.h` | `xmlHashAdd3` | `xml_hash_add3` |
| `hash.h` | `xmlHashAddEntry3` | `xml_hash_add_entry3` |
| `hash.h` | `xmlHashUpdateEntry3` | `xml_hash_update_entry3` |
| `hash.h` | `xmlHashRemoveEntry` | `xml_hash_remove_entry` |
| `hash.h` | `xmlHashRemoveEntry2` | `xml_hash_remove_entry2` |
| `hash.h` | `xmlHashRemoveEntry3` | `xml_hash_remove_entry3` |
| `hash.h` | `xmlHashLookup` | `xml_hash_lookup` |
| `hash.h` | `xmlHashLookup2` | `xml_hash_lookup2` |
| `hash.h` | `xmlHashLookup3` | `xml_hash_lookup3` |
| `hash.h` | `xmlHashQLookup` | `xml_hash_qlookup` |
| `hash.h` | `xmlHashQLookup2` | `xml_hash_qlookup2` |
| `hash.h` | `xmlHashQLookup3` | `xml_hash_qlookup3` |
| `hash.h` | `xmlHashCopySafe` | `xml_hash_copy_safe` |
| `hash.h` | `xmlHashCopy` | `xml_hash_copy` |
| `hash.h` | `xmlHashSize` | `xml_hash_size` |
| `hash.h` | `xmlHashScan` | `xml_hash_scan` |
| `hash.h` | `xmlHashScan3` | `xml_hash_scan3` |
| `hash.h` | `xmlHashScanFull` | `xml_hash_scan_full` |
| `hash.h` | `xmlHashScanFull3` | `xml_hash_scan_full3` |
| `HTMLparser.h` | `htmlNewParserCtxt` | `html_new_parser_ctxt` |
| `HTMLparser.h` | `htmlNewSAXParserCtxt` | `html_new_saxparser_ctxt` |
| `HTMLparser.h` | `htmlCreateMemoryParserCtxt` | `html_create_memory_parser_ctxt` |
| `HTMLparser.h` | `htmlParseDoc` | `html_parse_doc` |
| `HTMLparser.h` | `htmlCreatePushParserCtxt` | `html_create_push_parser_ctxt` |
| `HTMLparser.h` | `htmlParseChunk` | `html_parse_chunk` |
| `HTMLparser.h` | `htmlFreeParserCtxt` | `html_free_parser_ctxt` |
| `HTMLparser.h` | `htmlCtxtReset` | `html_ctxt_reset` |
| `HTMLparser.h` | `htmlCtxtSetOptions` | `html_ctxt_set_options` |
| `HTMLparser.h` | `htmlCtxtUseOptions` | `html_ctxt_use_options` |
| `HTMLparser.h` | `htmlReadDoc` | `html_read_doc` |
| `HTMLparser.h` | `htmlReadFile` | `html_read_file` |
| `HTMLparser.h` | `htmlReadMemory` | `html_read_memory` |
| `HTMLparser.h` | `htmlReadFd` | `html_read_fd` |
| `HTMLparser.h` | `htmlReadIO` | `html_read_io` |
| `HTMLparser.h` | `htmlCtxtParseDocument` | `html_ctxt_parse_document` |
| `HTMLparser.h` | `htmlCtxtReadDoc` | `html_ctxt_read_doc` |
| `HTMLparser.h` | `htmlCtxtReadFile` | `html_ctxt_read_file` |
| `HTMLparser.h` | `htmlCtxtReadMemory` | `html_ctxt_read_memory` |
| `HTMLparser.h` | `htmlCtxtReadFd` | `html_ctxt_read_fd` |
| `HTMLparser.h` | `htmlCtxtReadIO` | `html_ctxt_read_io` |
| `HTMLtree.h` | `htmlNewDoc` | `html_new_doc` |
| `HTMLtree.h` | `htmlNewDocNoDtD` | `html_new_doc_no_dt_d` |
| `HTMLtree.h` | `htmlGetMetaEncoding` | `html_get_meta_encoding` |
| `HTMLtree.h` | `htmlSetMetaEncoding` | `html_set_meta_encoding` |
| `HTMLtree.h` | `htmlDocDumpMemory` | `html_doc_dump_memory` |
| `HTMLtree.h` | `htmlDocDumpMemoryFormat` | `html_doc_dump_memory_format` |
| `HTMLtree.h` | `htmlSaveFile` | `html_save_file` |
| `HTMLtree.h` | `htmlSaveFileEnc` | `html_save_file_enc` |
| `HTMLtree.h` | `htmlSaveFileFormat` | `html_save_file_format` |
| `HTMLtree.h` | `htmlNodeDump` | `html_node_dump` |
| `HTMLtree.h` | `htmlDocDump` | `html_doc_dump` |
| `HTMLtree.h` | `htmlNodeDumpFile` | `html_node_dump_file` |
| `HTMLtree.h` | `htmlNodeDumpFileFormat` | `html_node_dump_file_format` |
| `HTMLtree.h` | `htmlNodeDumpOutput` | `html_node_dump_output` |
| `HTMLtree.h` | `htmlNodeDumpFormatOutput` | `html_node_dump_format_output` |
| `HTMLtree.h` | `htmlDocContentDumpOutput` | `html_doc_content_dump_output` |
| `HTMLtree.h` | `htmlDocContentDumpFormatOutput` | `html_doc_content_dump_format_output` |
| `list.h` | `xmlListCreate` | `xml_list_create` |
| `list.h` | `xmlListDelete` | `xml_list_delete` |
| `list.h` | `xmlListSearch` | `xml_list_search` |
| `list.h` | `xmlListReverseSearch` | `xml_list_reverse_search` |
| `list.h` | `xmlListInsert` | `xml_list_insert` |
| `list.h` | `xmlListAppend` | `xml_list_append` |
| `list.h` | `xmlListRemoveFirst` | `xml_list_remove_first` |
| `list.h` | `xmlListRemoveLast` | `xml_list_remove_last` |
| `list.h` | `xmlListRemoveAll` | `xml_list_remove_all` |
| `list.h` | `xmlListClear` | `xml_list_clear` |
| `list.h` | `xmlListEmpty` | `xml_list_empty` |
| `list.h` | `xmlListFront` | `xml_list_front` |
| `list.h` | `xmlListEnd` | `xml_list_end` |
| `list.h` | `xmlListSize` | `xml_list_size` |
| `list.h` | `xmlListPopFront` | `xml_list_pop_front` |
| `list.h` | `xmlListPopBack` | `xml_list_pop_back` |
| `list.h` | `xmlListPushFront` | `xml_list_push_front` |
| `list.h` | `xmlListPushBack` | `xml_list_push_back` |
| `list.h` | `xmlListReverse` | `xml_list_reverse` |
| `list.h` | `xmlListSort` | `xml_list_sort` |
| `list.h` | `xmlListWalk` | `xml_list_walk` |
| `list.h` | `xmlListReverseWalk` | `xml_list_reverse_walk` |
| `list.h` | `xmlListMerge` | `xml_list_merge` |
| `list.h` | `xmlListDup` | `xml_list_dup` |
| `list.h` | `xmlListCopy` | `xml_list_copy` |
| `list.h` | `xmlLinkGetData` | `xml_link_get_data` |
| `parser.h` | `xmlParserInputGrow` | `xml_parser_input_grow` |
| `parser.h` | `xmlParseDoc` | `xml_parse_doc` |
| `parser.h` | `xmlParseFile` | `xml_parse_file` |
| `parser.h` | `xmlCtxtParseDtd` | `xml_ctxt_parse_dtd` |
| `parser.h` | `xmlCtxtValidateDocument` | `xml_ctxt_validate_document` |
| `parser.h` | `xmlParseDTD` | `xml_parse_dtd` |
| `parser.h` | `xmlIOParseDTD` | `xml_io_parse_dtd` |
| `parser.h` | `xmlParseBalancedChunkMemory` | `xml_parse_balanced_chunk_memory` |
| `parser.h` | `xmlParseInNodeContext` | `xml_parse_in_node_context` |
| `parser.h` | `xmlParseBalancedChunkMemoryRecover` | `xml_parse_balanced_chunk_memory_recover` |
| `parser.h` | `xmlParseCtxtExternalEntity` | `xml_parse_ctxt_external_entity` |
| `parser.h` | `xmlNewParserCtxt` | `xml_new_parser_ctxt` |
| `parser.h` | `xmlNewSAXParserCtxt` | `xml_new_saxparser_ctxt` |
| `parser.h` | `xmlCreateDocParserCtxt` | `xml_create_doc_parser_ctxt` |
| `parser.h` | `xmlCreatePushParserCtxt` | `xml_create_push_parser_ctxt` |
| `parser.h` | `xmlParseChunk` | `xml_parse_chunk` |
| `parser.h` | `xmlCreateIOParserCtxt` | `xml_create_ioparser_ctxt` |
| `parser.h` | `xmlNewIOInputStream` | `xml_new_ioinput_stream` |
| `parser.h` | `xmlSetExternalEntityLoader` | `xml_set_external_entity_loader` |
| `parser.h` | `xmlGetExternalEntityLoader` | `xml_get_external_entity_loader` |
| `parser.h` | `xmlLoadExternalEntity` | `xml_load_external_entity` |
| `parser.h` | `xmlCtxtReset` | `xml_ctxt_reset` |
| `parser.h` | `xmlCtxtResetPush` | `xml_ctxt_reset_push` |
| `parser.h` | `xmlCtxtGetOptions` | `xml_ctxt_get_options` |
| `parser.h` | `xmlCtxtSetOptions` | `xml_ctxt_set_options` |
| `parser.h` | `xmlCtxtUseOptions` | `xml_ctxt_use_options` |
| `parser.h` | `xmlCtxtGetPrivate` | `xml_ctxt_get_private` |
| `parser.h` | `xmlCtxtSetPrivate` | `xml_ctxt_set_private` |
| `parser.h` | `xmlCtxtGetCatalogs` | `xml_ctxt_get_catalogs` |
| `parser.h` | `xmlCtxtSetCatalogs` | `xml_ctxt_set_catalogs` |
| `parser.h` | `xmlCtxtGetDict` | `xml_ctxt_get_dict` |
| `parser.h` | `xmlCtxtSetDict` | `xml_ctxt_set_dict` |
| `parser.h` | `xmlCtxtGetSaxHandler` | `xml_ctxt_get_sax_handler` |
| `parser.h` | `xmlCtxtSetSaxHandler` | `xml_ctxt_set_sax_handler` |
| `parser.h` | `xmlCtxtGetDocument` | `xml_ctxt_get_document` |
| `parser.h` | `xmlCtxtIsHtml` | `xml_ctxt_is_html` |
| `parser.h` | `xmlCtxtIsStopped` | `xml_ctxt_is_stopped` |
| `parser.h` | `xmlCtxtIsInSubset` | `xml_ctxt_is_in_subset` |
| `parser.h` | `xmlCtxtGetValidCtxt` | `xml_ctxt_get_valid_ctxt` |
| `parser.h` | `xmlCtxtGetVersion` | `xml_ctxt_get_version` |
| `parser.h` | `xmlCtxtGetDeclaredEncoding` | `xml_ctxt_get_declared_encoding` |
| `parser.h` | `xmlCtxtGetStandalone` | `xml_ctxt_get_standalone` |
| `parser.h` | `xmlCtxtGetStatus` | `xml_ctxt_get_status` |
| `parser.h` | `xmlCtxtGetUserData` | `xml_ctxt_get_user_data` |
| `parser.h` | `xmlCtxtGetNode` | `xml_ctxt_get_node` |
| `parser.h` | `xmlCtxtGetDocTypeDecl` | `xml_ctxt_get_doc_type_decl` |
| `parser.h` | `xmlCtxtGetInputPosition` | `xml_ctxt_get_input_position` |
| `parser.h` | `xmlCtxtGetInputWindow` | `xml_ctxt_get_input_window` |
| `parser.h` | `xmlCtxtSetErrorHandler` | `xml_ctxt_set_error_handler` |
| `parser.h` | `xmlCtxtSetResourceLoader` | `xml_ctxt_set_resource_loader` |
| `parser.h` | `xmlCtxtSetCharEncConvImpl` | `xml_ctxt_set_char_enc_conv_impl` |
| `parser.h` | `xmlCtxtSetMaxAmplification` | `xml_ctxt_set_max_amplification` |
| `parser.h` | `xmlReadDoc` | `xml_read_doc` |
| `parser.h` | `xmlReadFile` | `xml_read_file` |
| `parser.h` | `xmlReadMemory` | `xml_read_memory` |
| `parser.h` | `xmlReadFd` | `xml_read_fd` |
| `parser.h` | `xmlReadIO` | `xml_read_io` |
| `parser.h` | `xmlCtxtParseDocument` | `xml_ctxt_parse_document` |
| `parser.h` | `xmlCtxtParseContent` | `xml_ctxt_parse_content` |
| `parser.h` | `xmlCtxtReadDoc` | `xml_ctxt_read_doc` |
| `parser.h` | `xmlCtxtReadFile` | `xml_ctxt_read_file` |
| `parser.h` | `xmlCtxtReadMemory` | `xml_ctxt_read_memory` |
| `parser.h` | `xmlCtxtReadFd` | `xml_ctxt_read_fd` |
| `parser.h` | `xmlCtxtReadIO` | `xml_ctxt_read_io` |
| `parser.h` | `xmlNewInputFromUrl` | `xml_new_input_from_url` |
| `parser.h` | `xmlNewInputFromMemory` | `xml_new_input_from_memory` |
| `parser.h` | `xmlNewInputFromString` | `xml_new_input_from_string` |
| `parser.h` | `xmlNewInputFromFd` | `xml_new_input_from_fd` |
| `parser.h` | `xmlNewInputFromIO` | `xml_new_input_from_io` |
| `parser.h` | `xmlInputSetEncodingHandler` | `xml_input_set_encoding_handler` |
| `parser.h` | `xmlHasFeature` | `xml_has_feature` |
| `parserInternals.h` | `xmlCreateFileParserCtxt` | `xml_create_file_parser_ctxt` |
| `parserInternals.h` | `xmlCreateURLParserCtxt` | `xml_create_urlparser_ctxt` |
| `parserInternals.h` | `xmlCtxtErrMemory` | `xml_ctxt_err_memory` |
| `parserInternals.h` | `xmlSwitchEncoding` | `xml_switch_encoding` |
| `parserInternals.h` | `xmlSwitchEncodingName` | `xml_switch_encoding_name` |
| `parserInternals.h` | `xmlCtxtPushInput` | `xml_ctxt_push_input` |
| `parserInternals.h` | `xmlFreeInputStream` | `xml_free_input_stream` |
| `parserInternals.h` | `xmlNewInputFromFile` | `xml_new_input_from_file` |
| `parserInternals.h` | `xmlNewInputStream` | `xml_new_input_stream` |
| `parserInternals.h` | `xmlSplitQName` | `xml_split_qname` |
| `pattern.h` | `xmlFreePattern` | `xml_free_pattern` |
| `pattern.h` | `xmlFreePatternList` | `xml_free_pattern_list` |
| `pattern.h` | `xmlPatterncompile` | `xml_patterncompile` |
| `pattern.h` | `xmlPatternCompileSafe` | `xml_pattern_compile_safe` |
| `pattern.h` | `xmlPatternMatch` | `xml_pattern_match` |
| `pattern.h` | `xmlPatternStreamable` | `xml_pattern_streamable` |
| `pattern.h` | `xmlPatternMaxDepth` | `xml_pattern_max_depth` |
| `pattern.h` | `xmlPatternMinDepth` | `xml_pattern_min_depth` |
| `pattern.h` | `xmlPatternFromRoot` | `xml_pattern_from_root` |
| `pattern.h` | `xmlPatternGetStreamCtxt` | `xml_pattern_get_stream_ctxt` |
| `pattern.h` | `xmlFreeStreamCtxt` | `xml_free_stream_ctxt` |
| `pattern.h` | `xmlStreamPushNode` | `xml_stream_push_node` |
| `pattern.h` | `xmlStreamPush` | `xml_stream_push` |
| `pattern.h` | `xmlStreamPushAttr` | `xml_stream_push_attr` |
| `pattern.h` | `xmlStreamPop` | `xml_stream_pop` |
| `pattern.h` | `xmlStreamWantsAnyNode` | `xml_stream_wants_any_node` |
| `relaxng.h` | `xmlRelaxNGNewParserCtxt` | `xml_relax_ngnew_parser_ctxt` |
| `relaxng.h` | `xmlRelaxNGNewMemParserCtxt` | `xml_relax_ngnew_mem_parser_ctxt` |
| `relaxng.h` | `xmlRelaxNGNewDocParserCtxt` | `xml_relax_ngnew_doc_parser_ctxt` |
| `relaxng.h` | `xmlRelaxParserSetFlag` | `xml_relax_parser_set_flag` |
| `relaxng.h` | `xmlRelaxParserSetIncLImit` | `xml_relax_parser_set_inc_limit` |
| `relaxng.h` | `xmlRelaxNGFreeParserCtxt` | `xml_relax_ngfree_parser_ctxt` |
| `relaxng.h` | `xmlRelaxNGSetParserErrors` | `xml_relax_ngset_parser_errors` |
| `relaxng.h` | `xmlRelaxNGGetParserErrors` | `xml_relax_ngget_parser_errors` |
| `relaxng.h` | `xmlRelaxNGSetParserStructuredErrors` | `xml_relax_ngset_parser_structured_errors` |
| `relaxng.h` | `xmlRelaxNGSetResourceLoader` | `xml_relax_ngset_resource_loader` |
| `relaxng.h` | `xmlRelaxNGParse` | `xml_relax_ngparse` |
| `relaxng.h` | `xmlRelaxNGFree` | `xml_relax_ngfree` |
| `relaxng.h` | `xmlRelaxNGDump` | `xml_relax_ngdump` |
| `relaxng.h` | `xmlRelaxNGDumpTree` | `xml_relax_ngdump_tree` |
| `relaxng.h` | `xmlRelaxNGSetValidErrors` | `xml_relax_ngset_valid_errors` |
| `relaxng.h` | `xmlRelaxNGGetValidErrors` | `xml_relax_ngget_valid_errors` |
| `relaxng.h` | `xmlRelaxNGSetValidStructuredErrors` | `xml_relax_ngset_valid_structured_errors` |
| `relaxng.h` | `xmlRelaxNGNewValidCtxt` | `xml_relax_ngnew_valid_ctxt` |
| `relaxng.h` | `xmlRelaxNGFreeValidCtxt` | `xml_relax_ngfree_valid_ctxt` |
| `relaxng.h` | `xmlRelaxNGValidateDoc` | `xml_relax_ngvalidate_doc` |
| `relaxng.h` | `xmlRelaxNGValidatePushElement` | `xml_relax_ngvalidate_push_element` |
| `relaxng.h` | `xmlRelaxNGValidatePushCData` | `xml_relax_ngvalidate_push_cdata` |
| `relaxng.h` | `xmlRelaxNGValidatePopElement` | `xml_relax_ngvalidate_pop_element` |
| `relaxng.h` | `xmlRelaxNGValidateFullElement` | `xml_relax_ngvalidate_full_element` |
| `relaxng.h` | `xmlRelaxNGValidCtxtClearErrors` | `xml_relax_ngvalid_ctxt_clear_errors` |
| `SAX2.h` | `xmlSAX2GetPublicId` | `xml_sax2_get_public_id` |
| `SAX2.h` | `xmlSAX2GetSystemId` | `xml_sax2_get_system_id` |
| `SAX2.h` | `xmlSAX2SetDocumentLocator` | `xml_sax2_set_document_locator` |
| `SAX2.h` | `xmlSAX2GetLineNumber` | `xml_sax2_get_line_number` |
| `SAX2.h` | `xmlSAX2GetColumnNumber` | `xml_sax2_get_column_number` |
| `SAX2.h` | `xmlSAX2IsStandalone` | `xml_sax2_is_standalone` |
| `SAX2.h` | `xmlSAX2HasInternalSubset` | `xml_sax2_has_internal_subset` |
| `SAX2.h` | `xmlSAX2HasExternalSubset` | `xml_sax2_has_external_subset` |
| `SAX2.h` | `xmlSAX2InternalSubset` | `xml_sax2_internal_subset` |
| `SAX2.h` | `xmlSAX2ExternalSubset` | `xml_sax2_external_subset` |
| `SAX2.h` | `xmlSAX2GetEntity` | `xml_sax2_get_entity` |
| `SAX2.h` | `xmlSAX2GetParameterEntity` | `xml_sax2_get_parameter_entity` |
| `SAX2.h` | `xmlSAX2ResolveEntity` | `xml_sax2_resolve_entity` |
| `SAX2.h` | `xmlSAX2EntityDecl` | `xml_sax2_entity_decl` |
| `SAX2.h` | `xmlSAX2AttributeDecl` | `xml_sax2_attribute_decl` |
| `SAX2.h` | `xmlSAX2ElementDecl` | `xml_sax2_element_decl` |
| `SAX2.h` | `xmlSAX2NotationDecl` | `xml_sax2_notation_decl` |
| `SAX2.h` | `xmlSAX2UnparsedEntityDecl` | `xml_sax2_unparsed_entity_decl` |
| `SAX2.h` | `xmlSAX2StartElementNs` | `xml_sax2_start_element_ns` |
| `SAX2.h` | `xmlSAX2EndElementNs` | `xml_sax2_end_element_ns` |
| `SAX2.h` | `xmlSAX2Reference` | `xml_sax2_reference` |
| `SAX2.h` | `xmlSAX2Characters` | `xml_sax2_characters` |
| `SAX2.h` | `xmlSAX2IgnorableWhitespace` | `xml_sax2_ignorable_whitespace` |
| `SAX2.h` | `xmlSAX2ProcessingInstruction` | `xml_sax2_processing_instruction` |
| `SAX2.h` | `xmlSAX2Comment` | `xml_sax2_comment` |
| `SAX2.h` | `xmlSAX2CDataBlock` | `xml_sax2_cdata_block` |
| `SAX2.h` | `xmlSAXVersion` | `xml_sax_version` |
| `SAX2.h` | `xmlSAX2InitDefaultSAXHandler` | `xml_sax2_init_default_saxhandler` |
| `schemasInternals.h` | `xmlSchemaFreeType` | `xml_schema_free_type` |
| `schemasInternals.h` | `xmlSchemaFreeWildcard` | `xml_schema_free_wildcard` |
| `schematron.h` | `xmlSchematronNewParserCtxt` | `xml_schematron_new_parser_ctxt` |
| `schematron.h` | `xmlSchematronNewMemParserCtxt` | `xml_schematron_new_mem_parser_ctxt` |
| `schematron.h` | `xmlSchematronNewDocParserCtxt` | `xml_schematron_new_doc_parser_ctxt` |
| `schematron.h` | `xmlSchematronFreeParserCtxt` | `xml_schematron_free_parser_ctxt` |
| `schematron.h` | `xmlSchematronSetParserErrors` | `xml_schematron_set_parser_errors` |
| `schematron.h` | `xmlSchematronGetParserErrors` | `xml_schematron_get_parser_errors` |
| `schematron.h` | `xmlSchematronIsValid` | `xml_schematron_is_valid` |
| `schematron.h` | `xmlSchematronParse` | `xml_schematron_parse` |
| `schematron.h` | `xmlSchematronFree` | `xml_schematron_free` |
| `schematron.h` | `xmlSchematronSetValidStructuredErrors` | `xml_schematron_set_valid_structured_errors` |
| `schematron.h` | `xmlSchematronSetValidErrors` | `xml_schematron_set_valid_errors` |
| `schematron.h` | `xmlSchematronGetValidErrors` | `xml_schematron_get_valid_errors` |
| `schematron.h` | `xmlSchematronSetValidOptions` | `xml_schematron_set_valid_options` |
| `schematron.h` | `xmlSchematronValidCtxtGetOptions` | `xml_schematron_valid_ctxt_get_options` |
| `schematron.h` | `xmlSchematronValidateOneElement` | `xml_schematron_validate_one_element` |
| `schematron.h` | `xmlSchematronNewValidCtxt` | `xml_schematron_new_valid_ctxt` |
| `schematron.h` | `xmlSchematronFreeValidCtxt` | `xml_schematron_free_valid_ctxt` |
| `schematron.h` | `xmlSchematronValidateDoc` | `xml_schematron_validate_doc` |
| `threads.h` | `xmlCheckThreadLocalStorage` | `xml_check_thread_local_storage` |
| `threads.h` | `xmlNewMutex` | `xml_new_mutex` |
| `threads.h` | `xmlMutexLock` | `xml_mutex_lock` |
| `threads.h` | `xmlMutexUnlock` | `xml_mutex_unlock` |
| `threads.h` | `xmlFreeMutex` | `xml_free_mutex` |
| `threads.h` | `xmlNewRMutex` | `xml_new_rmutex` |
| `threads.h` | `xmlRMutexLock` | `xml_rmutex_lock` |
| `threads.h` | `xmlRMutexUnlock` | `xml_rmutex_unlock` |
| `threads.h` | `xmlFreeRMutex` | `xml_free_rmutex` |
| `tree.h` | `xmlValidateNCName` | `xml_validate_ncname` |
| `tree.h` | `xmlValidateQName` | `xml_validate_qname` |
| `tree.h` | `xmlValidateName` | `xml_validate_name` |
| `tree.h` | `xmlValidateNMToken` | `xml_validate_nmtoken` |
| `tree.h` | `xmlBuildQName` | `xml_build_qname` |
| `tree.h` | `xmlSplitQName2` | `xml_split_qname2` |
| `tree.h` | `xmlSplitQName3` | `xml_split_qname3` |
| `tree.h` | `xmlCreateIntSubset` | `xml_create_int_subset` |
| `tree.h` | `xmlNewDtd` | `xml_new_dtd` |
| `tree.h` | `xmlGetIntSubset` | `xml_get_int_subset` |
| `tree.h` | `xmlFreeDtd` | `xml_free_dtd` |
| `tree.h` | `xmlNewNs` | `xml_new_ns` |
| `tree.h` | `xmlFreeNs` | `xml_free_ns` |
| `tree.h` | `xmlFreeNsList` | `xml_free_ns_list` |
| `tree.h` | `xmlNewDoc` | `xml_new_doc` |
| `tree.h` | `xmlFreeDoc` | `xml_free_doc` |
| `tree.h` | `xmlNewDocProp` | `xml_new_doc_prop` |
| `tree.h` | `xmlNewProp` | `xml_new_prop` |
| `tree.h` | `xmlNewNsProp` | `xml_new_ns_prop` |
| `tree.h` | `xmlNewNsPropEatName` | `xml_new_ns_prop_eat_name` |
| `tree.h` | `xmlFreePropList` | `xml_free_prop_list` |
| `tree.h` | `xmlFreeProp` | `xml_free_prop` |
| `tree.h` | `xmlCopyProp` | `xml_copy_prop` |
| `tree.h` | `xmlCopyPropList` | `xml_copy_prop_list` |
| `tree.h` | `xmlCopyDtd` | `xml_copy_dtd` |
| `tree.h` | `xmlCopyDoc` | `xml_copy_doc` |
| `tree.h` | `xmlNewDocNode` | `xml_new_doc_node` |
| `tree.h` | `xmlNewDocNodeEatName` | `xml_new_doc_node_eat_name` |
| `tree.h` | `xmlNewNode` | `xml_new_node` |
| `tree.h` | `xmlNewNodeEatName` | `xml_new_node_eat_name` |
| `tree.h` | `xmlNewChild` | `xml_new_child` |
| `tree.h` | `xmlNewDocText` | `xml_new_doc_text` |
| `tree.h` | `xmlNewText` | `xml_new_text` |
| `tree.h` | `xmlNewDocPI` | `xml_new_doc_pi` |
| `tree.h` | `xmlNewPI` | `xml_new_pi` |
| `tree.h` | `xmlNewDocTextLen` | `xml_new_doc_text_len` |
| `tree.h` | `xmlNewTextLen` | `xml_new_text_len` |
| `tree.h` | `xmlNewDocComment` | `xml_new_doc_comment` |
| `tree.h` | `xmlNewComment` | `xml_new_comment` |
| `tree.h` | `xmlNewCDataBlock` | `xml_new_cdata_block` |
| `tree.h` | `xmlNewCharRef` | `xml_new_char_ref` |
| `tree.h` | `xmlNewReference` | `xml_new_reference` |
| `tree.h` | `xmlCopyNode` | `xml_copy_node` |
| `tree.h` | `xmlDocCopyNode` | `xml_doc_copy_node` |
| `tree.h` | `xmlDocCopyNodeList` | `xml_doc_copy_node_list` |
| `tree.h` | `xmlCopyNodeList` | `xml_copy_node_list` |
| `tree.h` | `xmlNewTextChild` | `xml_new_text_child` |
| `tree.h` | `xmlNewDocRawNode` | `xml_new_doc_raw_node` |
| `tree.h` | `xmlNewDocFragment` | `xml_new_doc_fragment` |
| `tree.h` | `xmlGetLineNo` | `xml_get_line_no` |
| `tree.h` | `xmlGetNodePath` | `xml_get_node_path` |
| `tree.h` | `xmlDocGetRootElement` | `xml_doc_get_root_element` |
| `tree.h` | `xmlGetLastChild` | `xml_get_last_child` |
| `tree.h` | `xmlNodeIsText` | `xml_node_is_text` |
| `tree.h` | `xmlIsBlankNode` | `xml_is_blank_node` |
| `tree.h` | `xmlDocSetRootElement` | `xml_doc_set_root_element` |
| `tree.h` | `xmlNodeSetName` | `xml_node_set_name` |
| `tree.h` | `xmlAddChild` | `xml_add_child` |
| `tree.h` | `xmlAddChildList` | `xml_add_child_list` |
| `tree.h` | `xmlReplaceNode` | `xml_replace_node` |
| `tree.h` | `xmlAddPrevSibling` | `xml_add_prev_sibling` |
| `tree.h` | `xmlAddSibling` | `xml_add_sibling` |
| `tree.h` | `xmlAddNextSibling` | `xml_add_next_sibling` |
| `tree.h` | `xmlUnlinkNode` | `xml_unlink_node` |
| `tree.h` | `xmlTextMerge` | `xml_text_merge` |
| `tree.h` | `xmlTextConcat` | `xml_text_concat` |
| `tree.h` | `xmlFreeNodeList` | `xml_free_node_list` |
| `tree.h` | `xmlFreeNode` | `xml_free_node` |
| `tree.h` | `xmlSetTreeDoc` | `xml_set_tree_doc` |
| `tree.h` | `xmlSetListDoc` | `xml_set_list_doc` |
| `tree.h` | `xmlSearchNs` | `xml_search_ns` |
| `tree.h` | `xmlSearchNsByHref` | `xml_search_ns_by_href` |
| `tree.h` | `xmlGetNsListSafe` | `xml_get_ns_list_safe` |
| `tree.h` | `xmlGetNsList` | `xml_get_ns_list` |
| `tree.h` | `xmlSetNs` | `xml_set_ns` |
| `tree.h` | `xmlCopyNamespace` | `xml_copy_namespace` |
| `tree.h` | `xmlCopyNamespaceList` | `xml_copy_namespace_list` |
| `tree.h` | `xmlSetProp` | `xml_set_prop` |
| `tree.h` | `xmlSetNsProp` | `xml_set_ns_prop` |
| `tree.h` | `xmlNodeGetAttrValue` | `xml_node_get_attr_value` |
| `tree.h` | `xmlGetNoNsProp` | `xml_get_no_ns_prop` |
| `tree.h` | `xmlGetProp` | `xml_get_prop` |
| `tree.h` | `xmlHasProp` | `xml_has_prop` |
| `tree.h` | `xmlHasNsProp` | `xml_has_ns_prop` |
| `tree.h` | `xmlGetNsProp` | `xml_get_ns_prop` |
| `tree.h` | `xmlStringGetNodeList` | `xml_string_get_node_list` |
| `tree.h` | `xmlStringLenGetNodeList` | `xml_string_len_get_node_list` |
| `tree.h` | `xmlNodeListGetString` | `xml_node_list_get_string` |
| `tree.h` | `xmlNodeListGetRawString` | `xml_node_list_get_raw_string` |
| `tree.h` | `xmlNodeSetContent` | `xml_node_set_content` |
| `tree.h` | `xmlNodeSetContentLen` | `xml_node_set_content_len` |
| `tree.h` | `xmlNodeAddContent` | `xml_node_add_content` |
| `tree.h` | `xmlNodeAddContentLen` | `xml_node_add_content_len` |
| `tree.h` | `xmlNodeGetContent` | `xml_node_get_content` |
| `tree.h` | `xmlNodeBufGetContent` | `xml_node_buf_get_content` |
| `tree.h` | `xmlBufGetNodeContent` | `xml_buf_get_node_content` |
| `tree.h` | `xmlNodeGetLang` | `xml_node_get_lang` |
| `tree.h` | `xmlNodeGetSpacePreserve` | `xml_node_get_space_preserve` |
| `tree.h` | `xmlNodeSetLang` | `xml_node_set_lang` |
| `tree.h` | `xmlNodeSetSpacePreserve` | `xml_node_set_space_preserve` |
| `tree.h` | `xmlNodeGetBaseSafe` | `xml_node_get_base_safe` |
| `tree.h` | `xmlNodeGetBase` | `xml_node_get_base` |
| `tree.h` | `xmlNodeSetBase` | `xml_node_set_base` |
| `tree.h` | `xmlRemoveProp` | `xml_remove_prop` |
| `tree.h` | `xmlUnsetNsProp` | `xml_unset_ns_prop` |
| `tree.h` | `xmlUnsetProp` | `xml_unset_prop` |
| `tree.h` | `xmlAttrSerializeTxtContent` | `xml_attr_serialize_txt_content` |
| `tree.h` | `xmlReconciliateNs` | `xml_reconciliate_ns` |
| `tree.h` | `xmlDocDumpFormatMemory` | `xml_doc_dump_format_memory` |
| `tree.h` | `xmlDocDumpMemory` | `xml_doc_dump_memory` |
| `tree.h` | `xmlDocDumpMemoryEnc` | `xml_doc_dump_memory_enc` |
| `tree.h` | `xmlDocDumpFormatMemoryEnc` | `xml_doc_dump_format_memory_enc` |
| `tree.h` | `xmlDocFormatDump` | `xml_doc_format_dump` |
| `tree.h` | `xmlDocDump` | `xml_doc_dump` |
| `tree.h` | `xmlElemDump` | `xml_elem_dump` |
| `tree.h` | `xmlSaveFile` | `xml_save_file` |
| `tree.h` | `xmlSaveFormatFile` | `xml_save_format_file` |
| `tree.h` | `xmlBufNodeDump` | `xml_buf_node_dump` |
| `tree.h` | `xmlNodeDump` | `xml_node_dump` |
| `tree.h` | `xmlSaveFileTo` | `xml_save_file_to` |
| `tree.h` | `xmlSaveFormatFileTo` | `xml_save_format_file_to` |
| `tree.h` | `xmlNodeDumpOutput` | `xml_node_dump_output` |
| `tree.h` | `xmlSaveFormatFileEnc` | `xml_save_format_file_enc` |
| `tree.h` | `xmlSaveFileEnc` | `xml_save_file_enc` |
| `tree.h` | `xmlIsXHTML` | `xml_is_xhtml` |
| `tree.h` | `xmlGetDocCompressMode` | `xml_get_doc_compress_mode` |
| `tree.h` | `xmlDOMWrapNewCtxt` | `xml_domwrap_new_ctxt` |
| `tree.h` | `xmlDOMWrapFreeCtxt` | `xml_domwrap_free_ctxt` |
| `tree.h` | `xmlDOMWrapReconcileNamespaces` | `xml_domwrap_reconcile_namespaces` |
| `tree.h` | `xmlDOMWrapAdoptNode` | `xml_domwrap_adopt_node` |
| `tree.h` | `xmlDOMWrapRemoveNode` | `xml_domwrap_remove_node` |
| `tree.h` | `xmlDOMWrapCloneNode` | `xml_domwrap_clone_node` |
| `tree.h` | `xmlChildElementCount` | `xml_child_element_count` |
| `tree.h` | `xmlNextElementSibling` | `xml_next_element_sibling` |
| `tree.h` | `xmlFirstElementChild` | `xml_first_element_child` |
| `tree.h` | `xmlLastElementChild` | `xml_last_element_child` |
| `tree.h` | `xmlBufferCreate` | `xml_buffer_create` |
| `tree.h` | `xmlBufferCreateSize` | `xml_buffer_create_size` |
| `tree.h` | `xmlBufferFree` | `xml_buffer_free` |
| `tree.h` | `xmlBufferDump` | `xml_buffer_dump` |
| `tree.h` | `xmlBufferAdd` | `xml_buffer_add` |
| `tree.h` | `xmlBufferAddHead` | `xml_buffer_add_head` |
| `tree.h` | `xmlBufferCat` | `xml_buffer_cat` |
| `tree.h` | `xmlBufferEmpty` | `xml_buffer_empty` |
| `tree.h` | `xmlBufferContent` | `xml_buffer_content` |
| `tree.h` | `xmlBufferDetach` | `xml_buffer_detach` |
| `tree.h` | `xmlBufferSetAllocationScheme` | `xml_buffer_set_allocation_scheme` |
| `tree.h` | `xmlBufferLength` | `xml_buffer_length` |
| `tree.h` | `xmlBufferWriteCHAR` | `xml_buffer_write_char` |
| `tree.h` | `xmlBufferWriteChar` | `xml_buffer_write_char` |
| `tree.h` | `xmlBufferWriteQuotedString` | `xml_buffer_write_quoted_string` |
| `tree.h` | `xmlBufContent` | `xml_buf_content` |
| `tree.h` | `xmlBufEnd` | `xml_buf_end` |
| `tree.h` | `xmlBufUse` | `xml_buf_use` |
| `tree.h` | `xmlBufShrink` | `xml_buf_shrink` |
| `uri.h` | `xmlCreateURI` | `xml_create_uri` |
| `uri.h` | `xmlBuildURISafe` | `xml_build_urisafe` |
| `uri.h` | `xmlBuildURI` | `xml_build_uri` |
| `uri.h` | `xmlBuildRelativeURISafe` | `xml_build_relative_urisafe` |
| `uri.h` | `xmlBuildRelativeURI` | `xml_build_relative_uri` |
| `uri.h` | `xmlParseURI` | `xml_parse_uri` |
| `uri.h` | `xmlParseURISafe` | `xml_parse_urisafe` |
| `uri.h` | `xmlParseURIRaw` | `xml_parse_uriraw` |
| `uri.h` | `xmlParseURIReference` | `xml_parse_urireference` |
| `uri.h` | `xmlSaveUri` | `xml_save_uri` |
| `uri.h` | `xmlPrintURI` | `xml_print_uri` |
| `uri.h` | `xmlURIEscapeStr` | `xml_uri_escape_str` |
| `uri.h` | `xmlURIUnescapeString` | `xml_uri_unescape_string` |
| `uri.h` | `xmlNormalizeURIPath` | `xml_normalize_uripath` |
| `uri.h` | `xmlURIEscape` | `xml_uri_escape` |
| `uri.h` | `xmlFreeURI` | `xml_free_uri` |
| `uri.h` | `xmlCanonicPath` | `xml_canonic_path` |
| `uri.h` | `xmlPathToURI` | `xml_path_to_uri` |
| `valid.h` | `xmlAddNotationDecl` | `xml_add_notation_decl` |
| `valid.h` | `xmlAddElementDecl` | `xml_add_element_decl` |
| `valid.h` | `xmlAddAttributeDecl` | `xml_add_attribute_decl` |
| `valid.h` | `xmlAddIDSafe` | `xml_add_idsafe` |
| `valid.h` | `xmlAddID` | `xml_add_id` |
| `valid.h` | `xmlFreeIDTable` | `xml_free_idtable` |
| `valid.h` | `xmlGetID` | `xml_get_id` |
| `valid.h` | `xmlIsID` | `xml_is_id` |
| `valid.h` | `xmlRemoveID` | `xml_remove_id` |
| `valid.h` | `xmlNewValidCtxt` | `xml_new_valid_ctxt` |
| `valid.h` | `xmlValidateDocument` | `xml_validate_document` |
| `valid.h` | `xmlGetDtdAttrDesc` | `xml_get_dtd_attr_desc` |
| `valid.h` | `xmlGetDtdQAttrDesc` | `xml_get_dtd_qattr_desc` |
| `valid.h` | `xmlGetDtdNotationDesc` | `xml_get_dtd_notation_desc` |
| `valid.h` | `xmlGetDtdQElementDesc` | `xml_get_dtd_qelement_desc` |
| `valid.h` | `xmlGetDtdElementDesc` | `xml_get_dtd_element_desc` |
| `valid.h` | `xmlValidGetPotentialChildren` | `xml_valid_get_potential_children` |
| `valid.h` | `xmlValidGetValidElements` | `xml_valid_get_valid_elements` |
| `valid.h` | `xmlValidateNameValue` | `xml_validate_name_value` |
| `valid.h` | `xmlValidateNamesValue` | `xml_validate_names_value` |
| `valid.h` | `xmlValidateNmtokenValue` | `xml_validate_nmtoken_value` |
| `valid.h` | `xmlValidateNmtokensValue` | `xml_validate_nmtokens_value` |
| `xinclude.h` | `xmlXIncludeProcess` | `xml_xinclude_process` |
| `xinclude.h` | `xmlXIncludeProcessFlags` | `xml_xinclude_process_flags` |
| `xinclude.h` | `xmlXIncludeProcessFlagsData` | `xml_xinclude_process_flags_data` |
| `xinclude.h` | `xmlXIncludeProcessTreeFlagsData` | `xml_xinclude_process_tree_flags_data` |
| `xinclude.h` | `xmlXIncludeProcessTree` | `xml_xinclude_process_tree` |
| `xinclude.h` | `xmlXIncludeProcessTreeFlags` | `xml_xinclude_process_tree_flags` |
| `xinclude.h` | `xmlXIncludeNewContext` | `xml_xinclude_new_context` |
| `xinclude.h` | `xmlXIncludeSetFlags` | `xml_xinclude_set_flags` |
| `xinclude.h` | `xmlXIncludeSetErrorHandler` | `xml_xinclude_set_error_handler` |
| `xinclude.h` | `xmlXIncludeSetResourceLoader` | `xml_xinclude_set_resource_loader` |
| `xinclude.h` | `xmlXIncludeGetLastError` | `xml_xinclude_get_last_error` |
| `xinclude.h` | `xmlXIncludeFreeContext` | `xml_xinclude_free_context` |
| `xinclude.h` | `xmlXIncludeProcessNode` | `xml_xinclude_process_node` |
| `xmlerror.h` | `xmlSetGenericErrorFunc` | `xml_set_generic_error_func` |
| `xmlerror.h` | `xmlParserError` | `xml_parser_error` |
| `xmlerror.h` | `xmlParserWarning` | `xml_parser_warning` |
| `xmlerror.h` | `xmlParserValidityError` | `xml_parser_validity_error` |
| `xmlerror.h` | `xmlParserValidityWarning` | `xml_parser_validity_warning` |
| `xmlerror.h` | `xmlParserPrintFileInfo` | `xml_parser_print_file_info` |
| `xmlerror.h` | `xmlParserPrintFileContext` | `xml_parser_print_file_context` |
| `xmlerror.h` | `xmlFormatError` | `xml_format_error` |
| `xmlerror.h` | `xmlGetLastError` | `xml_get_last_error` |
| `xmlerror.h` | `xmlResetLastError` | `xml_reset_last_error` |
| `xmlerror.h` | `xmlCtxtGetLastError` | `xml_ctxt_get_last_error` |
| `xmlerror.h` | `xmlCtxtResetLastError` | `xml_ctxt_reset_last_error` |
| `xmlerror.h` | `xmlResetError` | `xml_reset_error` |
| `xmlerror.h` | `xmlCopyError` | `xml_copy_error` |
| `xmlexports.h` | `__declspec` | `__declspec` |
| `xmlexports.h` | `__declspec` | `__declspec` |
| `xmlexports.h` | `xmlCheckVersion` | `xml_check_version` |
| `xmlIO.h` | `xmlCleanupInputCallbacks` | `xml_cleanup_input_callbacks` |
| `xmlIO.h` | `xmlPopInputCallbacks` | `xml_pop_input_callbacks` |
| `xmlIO.h` | `xmlRegisterDefaultInputCallbacks` | `xml_register_default_input_callbacks` |
| `xmlIO.h` | `xmlAllocParserInputBuffer` | `xml_alloc_parser_input_buffer` |
| `xmlIO.h` | `xmlParserInputBufferCreateFd` | `xml_parser_input_buffer_create_fd` |
| `xmlIO.h` | `xmlParserInputBufferCreateMem` | `xml_parser_input_buffer_create_mem` |
| `xmlIO.h` | `xmlParserInputBufferCreateStatic` | `xml_parser_input_buffer_create_static` |
| `xmlIO.h` | `xmlParserInputBufferCreateIO` | `xml_parser_input_buffer_create_io` |
| `xmlIO.h` | `xmlFreeParserInputBuffer` | `xml_free_parser_input_buffer` |
| `xmlIO.h` | `xmlParserGetDirectory` | `xml_parser_get_directory` |
| `xmlIO.h` | `xmlRegisterInputCallbacks` | `xml_register_input_callbacks` |
| `xmlIO.h` | `__xmlParserInputBufferCreateFilename` | `__xml_parser_input_buffer_create_filename` |
| `xmlIO.h` | `xmlCleanupOutputCallbacks` | `xml_cleanup_output_callbacks` |
| `xmlIO.h` | `xmlPopOutputCallbacks` | `xml_pop_output_callbacks` |
| `xmlIO.h` | `xmlRegisterDefaultOutputCallbacks` | `xml_register_default_output_callbacks` |
| `xmlIO.h` | `xmlAllocOutputBuffer` | `xml_alloc_output_buffer` |
| `xmlIO.h` | `xmlOutputBufferCreateFilename` | `xml_output_buffer_create_filename` |
| `xmlIO.h` | `xmlOutputBufferCreateFile` | `xml_output_buffer_create_file` |
| `xmlIO.h` | `xmlOutputBufferCreateBuffer` | `xml_output_buffer_create_buffer` |
| `xmlIO.h` | `xmlOutputBufferCreateFd` | `xml_output_buffer_create_fd` |
| `xmlIO.h` | `xmlOutputBufferCreateIO` | `xml_output_buffer_create_io` |
| `xmlIO.h` | `xmlOutputBufferGetContent` | `xml_output_buffer_get_content` |
| `xmlIO.h` | `xmlOutputBufferGetSize` | `xml_output_buffer_get_size` |
| `xmlIO.h` | `xmlOutputBufferWrite` | `xml_output_buffer_write` |
| `xmlIO.h` | `xmlOutputBufferWriteString` | `xml_output_buffer_write_string` |
| `xmlIO.h` | `xmlOutputBufferWriteEscape` | `xml_output_buffer_write_escape` |
| `xmlIO.h` | `xmlOutputBufferFlush` | `xml_output_buffer_flush` |
| `xmlIO.h` | `xmlOutputBufferClose` | `xml_output_buffer_close` |
| `xmlIO.h` | `xmlRegisterOutputCallbacks` | `xml_register_output_callbacks` |
| `xmlIO.h` | `__xmlOutputBufferCreateFilename` | `__xml_output_buffer_create_filename` |
| `xmlIO.h` | `xmlParserInputBufferCreateFilenameDefault` | `xml_parser_input_buffer_create_filename_default` |
| `xmlmemory.h` | `xmlMemSetup` | `xml_mem_setup` |
| `xmlmemory.h` | `xmlMemGet` | `xml_mem_get` |
| `xmlmemory.h` | `xmlMemSize` | `xml_mem_size` |
| `xmlmemory.h` | `xmlMemMalloc` | `xml_mem_malloc` |
| `xmlmemory.h` | `xmlMemRealloc` | `xml_mem_realloc` |
| `xmlreader.h` | `xmlNewTextReader` | `xml_new_text_reader` |
| `xmlreader.h` | `xmlNewTextReaderFilename` | `xml_new_text_reader_filename` |
| `xmlreader.h` | `xmlFreeTextReader` | `xml_free_text_reader` |
| `xmlreader.h` | `xmlTextReaderSetup` | `xml_text_reader_setup` |
| `xmlreader.h` | `xmlTextReaderSetMaxAmplification` | `xml_text_reader_set_max_amplification` |
| `xmlreader.h` | `xmlTextReaderGetLastError` | `xml_text_reader_get_last_error` |
| `xmlreader.h` | `xmlTextReaderRead` | `xml_text_reader_read` |
| `xmlreader.h` | `xmlTextReaderReadInnerXml` | `xml_text_reader_read_inner_xml` |
| `xmlreader.h` | `xmlTextReaderReadOuterXml` | `xml_text_reader_read_outer_xml` |
| `xmlreader.h` | `xmlTextReaderReadString` | `xml_text_reader_read_string` |
| `xmlreader.h` | `xmlTextReaderReadAttributeValue` | `xml_text_reader_read_attribute_value` |
| `xmlreader.h` | `xmlTextReaderAttributeCount` | `xml_text_reader_attribute_count` |
| `xmlreader.h` | `xmlTextReaderDepth` | `xml_text_reader_depth` |
| `xmlreader.h` | `xmlTextReaderHasAttributes` | `xml_text_reader_has_attributes` |
| `xmlreader.h` | `xmlTextReaderHasValue` | `xml_text_reader_has_value` |
| `xmlreader.h` | `xmlTextReaderIsDefault` | `xml_text_reader_is_default` |
| `xmlreader.h` | `xmlTextReaderIsEmptyElement` | `xml_text_reader_is_empty_element` |
| `xmlreader.h` | `xmlTextReaderNodeType` | `xml_text_reader_node_type` |
| `xmlreader.h` | `xmlTextReaderQuoteChar` | `xml_text_reader_quote_char` |
| `xmlreader.h` | `xmlTextReaderReadState` | `xml_text_reader_read_state` |
| `xmlreader.h` | `xmlTextReaderIsNamespaceDecl` | `xml_text_reader_is_namespace_decl` |
| `xmlreader.h` | `xmlTextReaderConstBaseUri` | `xml_text_reader_const_base_uri` |
| `xmlreader.h` | `xmlTextReaderConstLocalName` | `xml_text_reader_const_local_name` |
| `xmlreader.h` | `xmlTextReaderConstName` | `xml_text_reader_const_name` |
| `xmlreader.h` | `xmlTextReaderConstNamespaceUri` | `xml_text_reader_const_namespace_uri` |
| `xmlreader.h` | `xmlTextReaderConstPrefix` | `xml_text_reader_const_prefix` |
| `xmlreader.h` | `xmlTextReaderConstXmlLang` | `xml_text_reader_const_xml_lang` |
| `xmlreader.h` | `xmlTextReaderConstString` | `xml_text_reader_const_string` |
| `xmlreader.h` | `xmlTextReaderConstValue` | `xml_text_reader_const_value` |
| `xmlreader.h` | `xmlTextReaderBaseUri` | `xml_text_reader_base_uri` |
| `xmlreader.h` | `xmlTextReaderLocalName` | `xml_text_reader_local_name` |
| `xmlreader.h` | `xmlTextReaderName` | `xml_text_reader_name` |
| `xmlreader.h` | `xmlTextReaderNamespaceUri` | `xml_text_reader_namespace_uri` |
| `xmlreader.h` | `xmlTextReaderPrefix` | `xml_text_reader_prefix` |
| `xmlreader.h` | `xmlTextReaderXmlLang` | `xml_text_reader_xml_lang` |
| `xmlreader.h` | `xmlTextReaderValue` | `xml_text_reader_value` |
| `xmlreader.h` | `xmlTextReaderClose` | `xml_text_reader_close` |
| `xmlreader.h` | `xmlTextReaderGetAttributeNo` | `xml_text_reader_get_attribute_no` |
| `xmlreader.h` | `xmlTextReaderGetAttribute` | `xml_text_reader_get_attribute` |
| `xmlreader.h` | `xmlTextReaderGetAttributeNs` | `xml_text_reader_get_attribute_ns` |
| `xmlreader.h` | `xmlTextReaderGetRemainder` | `xml_text_reader_get_remainder` |
| `xmlreader.h` | `xmlTextReaderLookupNamespace` | `xml_text_reader_lookup_namespace` |
| `xmlreader.h` | `xmlTextReaderMoveToAttributeNo` | `xml_text_reader_move_to_attribute_no` |
| `xmlreader.h` | `xmlTextReaderMoveToAttribute` | `xml_text_reader_move_to_attribute` |
| `xmlreader.h` | `xmlTextReaderMoveToAttributeNs` | `xml_text_reader_move_to_attribute_ns` |
| `xmlreader.h` | `xmlTextReaderMoveToFirstAttribute` | `xml_text_reader_move_to_first_attribute` |
| `xmlreader.h` | `xmlTextReaderMoveToNextAttribute` | `xml_text_reader_move_to_next_attribute` |
| `xmlreader.h` | `xmlTextReaderMoveToElement` | `xml_text_reader_move_to_element` |
| `xmlreader.h` | `xmlTextReaderNormalization` | `xml_text_reader_normalization` |
| `xmlreader.h` | `xmlTextReaderConstEncoding` | `xml_text_reader_const_encoding` |
| `xmlreader.h` | `xmlTextReaderSetParserProp` | `xml_text_reader_set_parser_prop` |
| `xmlreader.h` | `xmlTextReaderGetParserProp` | `xml_text_reader_get_parser_prop` |
| `xmlreader.h` | `xmlTextReaderCurrentNode` | `xml_text_reader_current_node` |
| `xmlreader.h` | `xmlTextReaderGetParserLineNumber` | `xml_text_reader_get_parser_line_number` |
| `xmlreader.h` | `xmlTextReaderGetParserColumnNumber` | `xml_text_reader_get_parser_column_number` |
| `xmlreader.h` | `xmlTextReaderPreserve` | `xml_text_reader_preserve` |
| `xmlreader.h` | `xmlTextReaderPreservePattern` | `xml_text_reader_preserve_pattern` |
| `xmlreader.h` | `xmlTextReaderCurrentDoc` | `xml_text_reader_current_doc` |
| `xmlreader.h` | `xmlTextReaderExpand` | `xml_text_reader_expand` |
| `xmlreader.h` | `xmlTextReaderNext` | `xml_text_reader_next` |
| `xmlreader.h` | `xmlTextReaderNextSibling` | `xml_text_reader_next_sibling` |
| `xmlreader.h` | `xmlTextReaderIsValid` | `xml_text_reader_is_valid` |
| `xmlreader.h` | `xmlTextReaderRelaxNGValidate` | `xml_text_reader_relax_ngvalidate` |
| `xmlreader.h` | `xmlTextReaderRelaxNGValidateCtxt` | `xml_text_reader_relax_ngvalidate_ctxt` |
| `xmlreader.h` | `xmlTextReaderRelaxNGSetSchema` | `xml_text_reader_relax_ngset_schema` |
| `xmlreader.h` | `xmlTextReaderSchemaValidate` | `xml_text_reader_schema_validate` |
| `xmlreader.h` | `xmlTextReaderSchemaValidateCtxt` | `xml_text_reader_schema_validate_ctxt` |
| `xmlreader.h` | `xmlTextReaderSetSchema` | `xml_text_reader_set_schema` |
| `xmlreader.h` | `xmlTextReaderConstXmlVersion` | `xml_text_reader_const_xml_version` |
| `xmlreader.h` | `xmlTextReaderStandalone` | `xml_text_reader_standalone` |
| `xmlreader.h` | `xmlTextReaderByteConsumed` | `xml_text_reader_byte_consumed` |
| `xmlreader.h` | `xmlReaderWalker` | `xml_reader_walker` |
| `xmlreader.h` | `xmlReaderForDoc` | `xml_reader_for_doc` |
| `xmlreader.h` | `xmlReaderForFile` | `xml_reader_for_file` |
| `xmlreader.h` | `xmlReaderForMemory` | `xml_reader_for_memory` |
| `xmlreader.h` | `xmlReaderForFd` | `xml_reader_for_fd` |
| `xmlreader.h` | `xmlReaderForIO` | `xml_reader_for_io` |
| `xmlreader.h` | `xmlReaderNewWalker` | `xml_reader_new_walker` |
| `xmlreader.h` | `xmlReaderNewDoc` | `xml_reader_new_doc` |
| `xmlreader.h` | `xmlReaderNewFile` | `xml_reader_new_file` |
| `xmlreader.h` | `xmlReaderNewMemory` | `xml_reader_new_memory` |
| `xmlreader.h` | `xmlReaderNewFd` | `xml_reader_new_fd` |
| `xmlreader.h` | `xmlReaderNewIO` | `xml_reader_new_io` |
| `xmlreader.h` | `xmlTextReaderLocatorLineNumber` | `xml_text_reader_locator_line_number` |
| `xmlreader.h` | `xmlTextReaderLocatorBaseURI` | `xml_text_reader_locator_base_uri` |
| `xmlreader.h` | `xmlTextReaderSetErrorHandler` | `xml_text_reader_set_error_handler` |
| `xmlreader.h` | `xmlTextReaderSetStructuredErrorHandler` | `xml_text_reader_set_structured_error_handler` |
| `xmlreader.h` | `xmlTextReaderGetErrorHandler` | `xml_text_reader_get_error_handler` |
| `xmlreader.h` | `xmlTextReaderSetResourceLoader` | `xml_text_reader_set_resource_loader` |
| `xmlregexp.h` | `xmlRegexpCompile` | `xml_regexp_compile` |
| `xmlregexp.h` | `xmlRegexpIsDeterminist` | `xml_regexp_is_determinist` |
| `xmlsave.h` | `xmlSaveToFd` | `xml_save_to_fd` |
| `xmlsave.h` | `xmlSaveToFilename` | `xml_save_to_filename` |
| `xmlsave.h` | `xmlSaveToBuffer` | `xml_save_to_buffer` |
| `xmlsave.h` | `xmlSaveToIO` | `xml_save_to_io` |
| `xmlsave.h` | `xmlSaveDoc` | `xml_save_doc` |
| `xmlsave.h` | `xmlSaveTree` | `xml_save_tree` |
| `xmlsave.h` | `xmlSaveFlush` | `xml_save_flush` |
| `xmlsave.h` | `xmlSaveClose` | `xml_save_close` |
| `xmlsave.h` | `xmlSaveFinish` | `xml_save_finish` |
| `xmlschemas.h` | `xmlSchemaNewParserCtxt` | `xml_schema_new_parser_ctxt` |
| `xmlschemas.h` | `xmlSchemaNewMemParserCtxt` | `xml_schema_new_mem_parser_ctxt` |
| `xmlschemas.h` | `xmlSchemaNewDocParserCtxt` | `xml_schema_new_doc_parser_ctxt` |
| `xmlschemas.h` | `xmlSchemaFreeParserCtxt` | `xml_schema_free_parser_ctxt` |
| `xmlschemas.h` | `xmlSchemaSetParserErrors` | `xml_schema_set_parser_errors` |
| `xmlschemas.h` | `xmlSchemaSetParserStructuredErrors` | `xml_schema_set_parser_structured_errors` |
| `xmlschemas.h` | `xmlSchemaGetParserErrors` | `xml_schema_get_parser_errors` |
| `xmlschemas.h` | `xmlSchemaSetResourceLoader` | `xml_schema_set_resource_loader` |
| `xmlschemas.h` | `xmlSchemaIsValid` | `xml_schema_is_valid` |
| `xmlschemas.h` | `xmlSchemaParse` | `xml_schema_parse` |
| `xmlschemas.h` | `xmlSchemaFree` | `xml_schema_free` |
| `xmlschemas.h` | `xmlSchemaDump` | `xml_schema_dump` |
| `xmlschemas.h` | `xmlSchemaSetValidErrors` | `xml_schema_set_valid_errors` |
| `xmlschemas.h` | `xmlSchemaSetValidStructuredErrors` | `xml_schema_set_valid_structured_errors` |
| `xmlschemas.h` | `xmlSchemaGetValidErrors` | `xml_schema_get_valid_errors` |
| `xmlschemas.h` | `xmlSchemaSetValidOptions` | `xml_schema_set_valid_options` |
| `xmlschemas.h` | `xmlSchemaValidateSetFilename` | `xml_schema_validate_set_filename` |
| `xmlschemas.h` | `xmlSchemaValidCtxtGetOptions` | `xml_schema_valid_ctxt_get_options` |
| `xmlschemas.h` | `xmlSchemaNewValidCtxt` | `xml_schema_new_valid_ctxt` |
| `xmlschemas.h` | `xmlSchemaFreeValidCtxt` | `xml_schema_free_valid_ctxt` |
| `xmlschemas.h` | `xmlSchemaValidateDoc` | `xml_schema_validate_doc` |
| `xmlschemas.h` | `xmlSchemaValidateOneElement` | `xml_schema_validate_one_element` |
| `xmlschemas.h` | `xmlSchemaValidateStream` | `xml_schema_validate_stream` |
| `xmlschemas.h` | `xmlSchemaValidateFile` | `xml_schema_validate_file` |
| `xmlschemas.h` | `xmlSchemaValidCtxtGetParserCtxt` | `xml_schema_valid_ctxt_get_parser_ctxt` |
| `xmlschemas.h` | `xmlSchemaSAXPlug` | `xml_schema_saxplug` |
| `xmlschemas.h` | `xmlSchemaSAXUnplug` | `xml_schema_saxunplug` |
| `xmlschemas.h` | `xmlSchemaValidateSetLocator` | `xml_schema_validate_set_locator` |
| `xmlschemastypes.h` | `xmlSchemaGetPredefinedType` | `xml_schema_get_predefined_type` |
| `xmlschemastypes.h` | `xmlSchemaValidatePredefinedType` | `xml_schema_validate_predefined_type` |
| `xmlschemastypes.h` | `xmlSchemaValPredefTypeNode` | `xml_schema_val_predef_type_node` |
| `xmlschemastypes.h` | `xmlSchemaValidateFacet` | `xml_schema_validate_facet` |
| `xmlschemastypes.h` | `xmlSchemaValidateFacetWhtsp` | `xml_schema_validate_facet_whtsp` |
| `xmlschemastypes.h` | `xmlSchemaFreeValue` | `xml_schema_free_value` |
| `xmlschemastypes.h` | `xmlSchemaNewFacet` | `xml_schema_new_facet` |
| `xmlschemastypes.h` | `xmlSchemaCheckFacet` | `xml_schema_check_facet` |
| `xmlschemastypes.h` | `xmlSchemaFreeFacet` | `xml_schema_free_facet` |
| `xmlschemastypes.h` | `xmlSchemaCompareValues` | `xml_schema_compare_values` |
| `xmlschemastypes.h` | `xmlSchemaGetBuiltInListSimpleTypeItemType` | `xml_schema_get_built_in_list_simple_type_item_type` |
| `xmlschemastypes.h` | `xmlSchemaValidateListSimpleTypeFacet` | `xml_schema_validate_list_simple_type_facet` |
| `xmlschemastypes.h` | `xmlSchemaGetBuiltInType` | `xml_schema_get_built_in_type` |
| `xmlschemastypes.h` | `xmlSchemaIsBuiltInTypeFacet` | `xml_schema_is_built_in_type_facet` |
| `xmlschemastypes.h` | `xmlSchemaCollapseString` | `xml_schema_collapse_string` |
| `xmlschemastypes.h` | `xmlSchemaWhiteSpaceReplace` | `xml_schema_white_space_replace` |
| `xmlschemastypes.h` | `xmlSchemaGetFacetValueAsULong` | `xml_schema_get_facet_value_as_ulong` |
| `xmlschemastypes.h` | `xmlSchemaValidateLengthFacet` | `xml_schema_validate_length_facet` |
| `xmlschemastypes.h` | `xmlSchemaValidateLengthFacetWhtsp` | `xml_schema_validate_length_facet_whtsp` |
| `xmlschemastypes.h` | `xmlSchemaValPredefTypeNodeNoNorm` | `xml_schema_val_predef_type_node_no_norm` |
| `xmlschemastypes.h` | `xmlSchemaGetCanonValue` | `xml_schema_get_canon_value` |
| `xmlschemastypes.h` | `xmlSchemaGetCanonValueWhtsp` | `xml_schema_get_canon_value_whtsp` |
| `xmlschemastypes.h` | `xmlSchemaValueAppend` | `xml_schema_value_append` |
| `xmlschemastypes.h` | `xmlSchemaValueGetNext` | `xml_schema_value_get_next` |
| `xmlschemastypes.h` | `xmlSchemaValueGetAsString` | `xml_schema_value_get_as_string` |
| `xmlschemastypes.h` | `xmlSchemaValueGetAsBoolean` | `xml_schema_value_get_as_boolean` |
| `xmlschemastypes.h` | `xmlSchemaNewStringValue` | `xml_schema_new_string_value` |
| `xmlschemastypes.h` | `xmlSchemaNewNOTATIONValue` | `xml_schema_new_notationvalue` |
| `xmlschemastypes.h` | `xmlSchemaNewQNameValue` | `xml_schema_new_qname_value` |
| `xmlschemastypes.h` | `xmlSchemaCompareValuesWhtsp` | `xml_schema_compare_values_whtsp` |
| `xmlschemastypes.h` | `xmlSchemaCopyValue` | `xml_schema_copy_value` |
| `xmlschemastypes.h` | `xmlSchemaGetValType` | `xml_schema_get_val_type` |
| `xmlstring.h` | `xmlStrdup` | `xml_strdup` |
| `xmlstring.h` | `xmlStrndup` | `xml_strndup` |
| `xmlstring.h` | `xmlCharStrndup` | `xml_char_strndup` |
| `xmlstring.h` | `xmlCharStrdup` | `xml_char_strdup` |
| `xmlstring.h` | `xmlStrsub` | `xml_strsub` |
| `xmlstring.h` | `xmlStrchr` | `xml_strchr` |
| `xmlstring.h` | `xmlStrstr` | `xml_strstr` |
| `xmlstring.h` | `xmlStrcasestr` | `xml_strcasestr` |
| `xmlstring.h` | `xmlStrcmp` | `xml_strcmp` |
| `xmlstring.h` | `xmlStrncmp` | `xml_strncmp` |
| `xmlstring.h` | `xmlStrcasecmp` | `xml_strcasecmp` |
| `xmlstring.h` | `xmlStrncasecmp` | `xml_strncasecmp` |
| `xmlstring.h` | `xmlStrEqual` | `xml_str_equal` |
| `xmlstring.h` | `xmlStrQEqual` | `xml_str_qequal` |
| `xmlstring.h` | `xmlStrlen` | `xml_strlen` |
| `xmlstring.h` | `xmlStrcat` | `xml_strcat` |
| `xmlstring.h` | `xmlStrncat` | `xml_strncat` |
| `xmlstring.h` | `xmlStrncatNew` | `xml_strncat_new` |
| `xmlstring.h` | `xmlStrPrintf` | `xml_str_printf` |
| `xmlstring.h` | `xmlStrVPrintf` | `xml_str_vprintf` |
| `xmlstring.h` | `xmlGetUTF8Char` | `xml_get_utf8_char` |
| `xmlstring.h` | `xmlCheckUTF8` | `xml_check_utf8` |
| `xmlstring.h` | `xmlUTF8Strsize` | `xml_utf8_strsize` |
| `xmlstring.h` | `xmlUTF8Strndup` | `xml_utf8_strndup` |
| `xmlstring.h` | `xmlUTF8Strpos` | `xml_utf8_strpos` |
| `xmlstring.h` | `xmlUTF8Strloc` | `xml_utf8_strloc` |
| `xmlstring.h` | `xmlUTF8Strsub` | `xml_utf8_strsub` |
| `xmlstring.h` | `xmlUTF8Strlen` | `xml_utf8_strlen` |
| `xmlstring.h` | `xmlUTF8Size` | `xml_utf8_size` |
| `xmlstring.h` | `xmlUTF8Charcmp` | `xml_utf8_charcmp` |
| `xmlwriter.h` | `xmlNewTextWriter` | `xml_new_text_writer` |
| `xmlwriter.h` | `xmlNewTextWriterFilename` | `xml_new_text_writer_filename` |
| `xmlwriter.h` | `xmlNewTextWriterMemory` | `xml_new_text_writer_memory` |
| `xmlwriter.h` | `xmlNewTextWriterPushParser` | `xml_new_text_writer_push_parser` |
| `xmlwriter.h` | `xmlNewTextWriterDoc` | `xml_new_text_writer_doc` |
| `xmlwriter.h` | `xmlNewTextWriterTree` | `xml_new_text_writer_tree` |
| `xmlwriter.h` | `xmlFreeTextWriter` | `xml_free_text_writer` |
| `xmlwriter.h` | `xmlTextWriterStartDocument` | `xml_text_writer_start_document` |
| `xmlwriter.h` | `xmlTextWriterEndDocument` | `xml_text_writer_end_document` |
| `xmlwriter.h` | `xmlTextWriterStartComment` | `xml_text_writer_start_comment` |
| `xmlwriter.h` | `xmlTextWriterEndComment` | `xml_text_writer_end_comment` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatComment` | `xml_text_writer_write_format_comment` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatComment` | `xml_text_writer_write_vformat_comment` |
| `xmlwriter.h` | `xmlTextWriterWriteComment` | `xml_text_writer_write_comment` |
| `xmlwriter.h` | `xmlTextWriterStartElement` | `xml_text_writer_start_element` |
| `xmlwriter.h` | `xmlTextWriterStartElementNS` | `xml_text_writer_start_element_ns` |
| `xmlwriter.h` | `xmlTextWriterEndElement` | `xml_text_writer_end_element` |
| `xmlwriter.h` | `xmlTextWriterFullEndElement` | `xml_text_writer_full_end_element` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatElement` | `xml_text_writer_write_format_element` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatElement` | `xml_text_writer_write_vformat_element` |
| `xmlwriter.h` | `xmlTextWriterWriteElement` | `xml_text_writer_write_element` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatElementNS` | `xml_text_writer_write_format_element_ns` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatElementNS` | `xml_text_writer_write_vformat_element_ns` |
| `xmlwriter.h` | `xmlTextWriterWriteElementNS` | `xml_text_writer_write_element_ns` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatRaw` | `xml_text_writer_write_format_raw` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatRaw` | `xml_text_writer_write_vformat_raw` |
| `xmlwriter.h` | `xmlTextWriterWriteRawLen` | `xml_text_writer_write_raw_len` |
| `xmlwriter.h` | `xmlTextWriterWriteRaw` | `xml_text_writer_write_raw` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatString` | `xml_text_writer_write_format_string` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatString` | `xml_text_writer_write_vformat_string` |
| `xmlwriter.h` | `xmlTextWriterWriteString` | `xml_text_writer_write_string` |
| `xmlwriter.h` | `xmlTextWriterWriteBase64` | `xml_text_writer_write_base64` |
| `xmlwriter.h` | `xmlTextWriterWriteBinHex` | `xml_text_writer_write_bin_hex` |
| `xmlwriter.h` | `xmlTextWriterStartAttribute` | `xml_text_writer_start_attribute` |
| `xmlwriter.h` | `xmlTextWriterStartAttributeNS` | `xml_text_writer_start_attribute_ns` |
| `xmlwriter.h` | `xmlTextWriterEndAttribute` | `xml_text_writer_end_attribute` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatAttribute` | `xml_text_writer_write_format_attribute` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatAttribute` | `xml_text_writer_write_vformat_attribute` |
| `xmlwriter.h` | `xmlTextWriterWriteAttribute` | `xml_text_writer_write_attribute` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatAttributeNS` | `xml_text_writer_write_format_attribute_ns` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatAttributeNS` | `xml_text_writer_write_vformat_attribute_ns` |
| `xmlwriter.h` | `xmlTextWriterWriteAttributeNS` | `xml_text_writer_write_attribute_ns` |
| `xmlwriter.h` | `xmlTextWriterStartPI` | `xml_text_writer_start_pi` |
| `xmlwriter.h` | `xmlTextWriterEndPI` | `xml_text_writer_end_pi` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatPI` | `xml_text_writer_write_format_pi` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatPI` | `xml_text_writer_write_vformat_pi` |
| `xmlwriter.h` | `xmlTextWriterWritePI` | `xml_text_writer_write_pi` |
| `xmlwriter.h` | `xmlTextWriterStartCDATA` | `xml_text_writer_start_cdata` |
| `xmlwriter.h` | `xmlTextWriterEndCDATA` | `xml_text_writer_end_cdata` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatCDATA` | `xml_text_writer_write_format_cdata` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatCDATA` | `xml_text_writer_write_vformat_cdata` |
| `xmlwriter.h` | `xmlTextWriterWriteCDATA` | `xml_text_writer_write_cdata` |
| `xmlwriter.h` | `xmlTextWriterStartDTD` | `xml_text_writer_start_dtd` |
| `xmlwriter.h` | `xmlTextWriterEndDTD` | `xml_text_writer_end_dtd` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatDTD` | `xml_text_writer_write_format_dtd` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatDTD` | `xml_text_writer_write_vformat_dtd` |
| `xmlwriter.h` | `xmlTextWriterWriteDTD` | `xml_text_writer_write_dtd` |
| `xmlwriter.h` | `xmlTextWriterStartDTDElement` | `xml_text_writer_start_dtdelement` |
| `xmlwriter.h` | `xmlTextWriterEndDTDElement` | `xml_text_writer_end_dtdelement` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatDTDElement` | `xml_text_writer_write_format_dtdelement` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatDTDElement` | `xml_text_writer_write_vformat_dtdelement` |
| `xmlwriter.h` | `xmlTextWriterWriteDTDElement` | `xml_text_writer_write_dtdelement` |
| `xmlwriter.h` | `xmlTextWriterStartDTDAttlist` | `xml_text_writer_start_dtdattlist` |
| `xmlwriter.h` | `xmlTextWriterEndDTDAttlist` | `xml_text_writer_end_dtdattlist` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatDTDAttlist` | `xml_text_writer_write_format_dtdattlist` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatDTDAttlist` | `xml_text_writer_write_vformat_dtdattlist` |
| `xmlwriter.h` | `xmlTextWriterWriteDTDAttlist` | `xml_text_writer_write_dtdattlist` |
| `xmlwriter.h` | `xmlTextWriterStartDTDEntity` | `xml_text_writer_start_dtdentity` |
| `xmlwriter.h` | `xmlTextWriterEndDTDEntity` | `xml_text_writer_end_dtdentity` |
| `xmlwriter.h` | `xmlTextWriterWriteFormatDTDInternalEntity` | `xml_text_writer_write_format_dtdinternal_entity` |
| `xmlwriter.h` | `xmlTextWriterWriteVFormatDTDInternalEntity` | `xml_text_writer_write_vformat_dtdinternal_entity` |
| `xmlwriter.h` | `xmlTextWriterWriteDTDInternalEntity` | `xml_text_writer_write_dtdinternal_entity` |
| `xmlwriter.h` | `xmlTextWriterWriteDTDExternalEntity` | `xml_text_writer_write_dtdexternal_entity` |
| `xmlwriter.h` | `xmlTextWriterWriteDTDExternalEntityContents` | `xml_text_writer_write_dtdexternal_entity_contents` |
| `xmlwriter.h` | `xmlTextWriterWriteDTDEntity` | `xml_text_writer_write_dtdentity` |
| `xmlwriter.h` | `xmlTextWriterWriteDTDNotation` | `xml_text_writer_write_dtdnotation` |
| `xmlwriter.h` | `xmlTextWriterSetIndent` | `xml_text_writer_set_indent` |
| `xmlwriter.h` | `xmlTextWriterSetIndentString` | `xml_text_writer_set_indent_string` |
| `xmlwriter.h` | `xmlTextWriterSetQuoteChar` | `xml_text_writer_set_quote_char` |
| `xmlwriter.h` | `xmlTextWriterFlush` | `xml_text_writer_flush` |
| `xmlwriter.h` | `xmlTextWriterClose` | `xml_text_writer_close` |
| `xpath.h` | `xmlXPathFreeObject` | `xml_xpath_free_object` |
| `xpath.h` | `xmlXPathNodeSetCreate` | `xml_xpath_node_set_create` |
| `xpath.h` | `xmlXPathFreeNodeSetList` | `xml_xpath_free_node_set_list` |
| `xpath.h` | `xmlXPathFreeNodeSet` | `xml_xpath_free_node_set` |
| `xpath.h` | `xmlXPathObjectCopy` | `xml_xpath_object_copy` |
| `xpath.h` | `xmlXPathCmpNodes` | `xml_xpath_cmp_nodes` |
| `xpath.h` | `xmlXPathCastNumberToBoolean` | `xml_xpath_cast_number_to_boolean` |
| `xpath.h` | `xmlXPathCastStringToBoolean` | `xml_xpath_cast_string_to_boolean` |
| `xpath.h` | `xmlXPathCastNodeSetToBoolean` | `xml_xpath_cast_node_set_to_boolean` |
| `xpath.h` | `xmlXPathCastToBoolean` | `xml_xpath_cast_to_boolean` |
| `xpath.h` | `xmlXPathCastBooleanToNumber` | `xml_xpath_cast_boolean_to_number` |
| `xpath.h` | `xmlXPathCastStringToNumber` | `xml_xpath_cast_string_to_number` |
| `xpath.h` | `xmlXPathCastNodeToNumber` | `xml_xpath_cast_node_to_number` |
| `xpath.h` | `xmlXPathCastNodeSetToNumber` | `xml_xpath_cast_node_set_to_number` |
| `xpath.h` | `xmlXPathCastToNumber` | `xml_xpath_cast_to_number` |
| `xpath.h` | `xmlXPathCastBooleanToString` | `xml_xpath_cast_boolean_to_string` |
| `xpath.h` | `xmlXPathCastNumberToString` | `xml_xpath_cast_number_to_string` |
| `xpath.h` | `xmlXPathCastNodeToString` | `xml_xpath_cast_node_to_string` |
| `xpath.h` | `xmlXPathCastNodeSetToString` | `xml_xpath_cast_node_set_to_string` |
| `xpath.h` | `xmlXPathCastToString` | `xml_xpath_cast_to_string` |
| `xpath.h` | `xmlXPathConvertBoolean` | `xml_xpath_convert_boolean` |
| `xpath.h` | `xmlXPathConvertNumber` | `xml_xpath_convert_number` |
| `xpath.h` | `xmlXPathConvertString` | `xml_xpath_convert_string` |
| `xpath.h` | `xmlXPathNewContext` | `xml_xpath_new_context` |
| `xpath.h` | `xmlXPathFreeContext` | `xml_xpath_free_context` |
| `xpath.h` | `xmlXPathSetErrorHandler` | `xml_xpath_set_error_handler` |
| `xpath.h` | `xmlXPathContextSetCache` | `xml_xpath_context_set_cache` |
| `xpath.h` | `xmlXPathOrderDocElems` | `xml_xpath_order_doc_elems` |
| `xpath.h` | `xmlXPathSetContextNode` | `xml_xpath_set_context_node` |
| `xpath.h` | `xmlXPathNodeEval` | `xml_xpath_node_eval` |
| `xpath.h` | `xmlXPathEval` | `xml_xpath_eval` |
| `xpath.h` | `xmlXPathEvalExpression` | `xml_xpath_eval_expression` |
| `xpath.h` | `xmlXPathEvalPredicate` | `xml_xpath_eval_predicate` |
| `xpath.h` | `xmlXPathCompile` | `xml_xpath_compile` |
| `xpath.h` | `xmlXPathCtxtCompile` | `xml_xpath_ctxt_compile` |
| `xpath.h` | `xmlXPathCompiledEval` | `xml_xpath_compiled_eval` |
| `xpath.h` | `xmlXPathCompiledEvalToBoolean` | `xml_xpath_compiled_eval_to_boolean` |
| `xpath.h` | `xmlXPathIsNaN` | `xml_xpath_is_na_n` |
| `xpath.h` | `xmlXPathIsInf` | `xml_xpath_is_inf` |
| `xpathInternals.h` | `xmlXPathPopBoolean` | `xml_xpath_pop_boolean` |
| `xpathInternals.h` | `xmlXPathPopNumber` | `xml_xpath_pop_number` |
| `xpathInternals.h` | `xmlXPathPopString` | `xml_xpath_pop_string` |
| `xpathInternals.h` | `xmlXPathPopNodeSet` | `xml_xpath_pop_node_set` |
| `xpathInternals.h` | `xmlXPathPopExternal` | `xml_xpath_pop_external` |
| `xpathInternals.h` | `xmlXPathRegisterVariableLookup` | `xml_xpath_register_variable_lookup` |
| `xpathInternals.h` | `xmlXPathRegisterFuncLookup` | `xml_xpath_register_func_lookup` |
| `xpathInternals.h` | `xmlXPatherror` | `xml_xpatherror` |
| `xpathInternals.h` | `xmlXPathErr` | `xml_xpath_err` |
| `xpathInternals.h` | `xmlXPathDebugDumpObject` | `xml_xpath_debug_dump_object` |
| `xpathInternals.h` | `xmlXPathDebugDumpCompExpr` | `xml_xpath_debug_dump_comp_expr` |
| `xpathInternals.h` | `xmlXPathNodeSetContains` | `xml_xpath_node_set_contains` |
| `xpathInternals.h` | `xmlXPathDifference` | `xml_xpath_difference` |
| `xpathInternals.h` | `xmlXPathIntersection` | `xml_xpath_intersection` |
| `xpathInternals.h` | `xmlXPathDistinctSorted` | `xml_xpath_distinct_sorted` |
| `xpathInternals.h` | `xmlXPathDistinct` | `xml_xpath_distinct` |
| `xpathInternals.h` | `xmlXPathHasSameNodes` | `xml_xpath_has_same_nodes` |
| `xpathInternals.h` | `xmlXPathNodeLeadingSorted` | `xml_xpath_node_leading_sorted` |
| `xpathInternals.h` | `xmlXPathLeadingSorted` | `xml_xpath_leading_sorted` |
| `xpathInternals.h` | `xmlXPathNodeLeading` | `xml_xpath_node_leading` |
| `xpathInternals.h` | `xmlXPathLeading` | `xml_xpath_leading` |
| `xpathInternals.h` | `xmlXPathNodeTrailingSorted` | `xml_xpath_node_trailing_sorted` |
| `xpathInternals.h` | `xmlXPathTrailingSorted` | `xml_xpath_trailing_sorted` |
| `xpathInternals.h` | `xmlXPathNodeTrailing` | `xml_xpath_node_trailing` |
| `xpathInternals.h` | `xmlXPathTrailing` | `xml_xpath_trailing` |
| `xpathInternals.h` | `xmlXPathRegisterNs` | `xml_xpath_register_ns` |
| `xpathInternals.h` | `xmlXPathNsLookup` | `xml_xpath_ns_lookup` |
| `xpathInternals.h` | `xmlXPathRegisteredNsCleanup` | `xml_xpath_registered_ns_cleanup` |
| `xpathInternals.h` | `xmlXPathRegisterFunc` | `xml_xpath_register_func` |
| `xpathInternals.h` | `xmlXPathRegisterFuncNS` | `xml_xpath_register_func_ns` |
| `xpathInternals.h` | `xmlXPathRegisterVariable` | `xml_xpath_register_variable` |
| `xpathInternals.h` | `xmlXPathRegisterVariableNS` | `xml_xpath_register_variable_ns` |
| `xpathInternals.h` | `xmlXPathFunctionLookup` | `xml_xpath_function_lookup` |
| `xpathInternals.h` | `xmlXPathFunctionLookupNS` | `xml_xpath_function_lookup_ns` |
| `xpathInternals.h` | `xmlXPathRegisteredFuncsCleanup` | `xml_xpath_registered_funcs_cleanup` |
| `xpathInternals.h` | `xmlXPathVariableLookup` | `xml_xpath_variable_lookup` |
| `xpathInternals.h` | `xmlXPathVariableLookupNS` | `xml_xpath_variable_lookup_ns` |
| `xpathInternals.h` | `xmlXPathRegisteredVariablesCleanup` | `xml_xpath_registered_variables_cleanup` |
| `xpathInternals.h` | `xmlXPathNewParserContext` | `xml_xpath_new_parser_context` |
| `xpathInternals.h` | `xmlXPathFreeParserContext` | `xml_xpath_free_parser_context` |
| `xpathInternals.h` | `xmlXPathValuePop` | `xml_xpath_value_pop` |
| `xpathInternals.h` | `xmlXPathValuePush` | `xml_xpath_value_push` |
| `xpathInternals.h` | `xmlXPathNewString` | `xml_xpath_new_string` |
| `xpathInternals.h` | `xmlXPathNewCString` | `xml_xpath_new_cstring` |
| `xpathInternals.h` | `xmlXPathWrapString` | `xml_xpath_wrap_string` |
| `xpathInternals.h` | `xmlXPathWrapCString` | `xml_xpath_wrap_cstring` |
| `xpathInternals.h` | `xmlXPathNewFloat` | `xml_xpath_new_float` |
| `xpathInternals.h` | `xmlXPathNewBoolean` | `xml_xpath_new_boolean` |
| `xpathInternals.h` | `xmlXPathNewNodeSet` | `xml_xpath_new_node_set` |
| `xpathInternals.h` | `xmlXPathNewValueTree` | `xml_xpath_new_value_tree` |
| `xpathInternals.h` | `xmlXPathNodeSetAdd` | `xml_xpath_node_set_add` |
| `xpathInternals.h` | `xmlXPathNodeSetAddUnique` | `xml_xpath_node_set_add_unique` |
| `xpathInternals.h` | `xmlXPathNodeSetAddNs` | `xml_xpath_node_set_add_ns` |
| `xpathInternals.h` | `xmlXPathNodeSetSort` | `xml_xpath_node_set_sort` |
| `xpathInternals.h` | `xmlXPathParseName` | `xml_xpath_parse_name` |
| `xpathInternals.h` | `xmlXPathParseNCName` | `xml_xpath_parse_ncname` |
| `xpathInternals.h` | `xmlXPathStringEvalNumber` | `xml_xpath_string_eval_number` |
| `xpathInternals.h` | `xmlXPathEvaluatePredicateResult` | `xml_xpath_evaluate_predicate_result` |
| `xpathInternals.h` | `xmlXPathRegisterAllFunctions` | `xml_xpath_register_all_functions` |
| `xpathInternals.h` | `xmlXPathNodeSetMerge` | `xml_xpath_node_set_merge` |
| `xpathInternals.h` | `xmlXPathNodeSetDel` | `xml_xpath_node_set_del` |
| `xpathInternals.h` | `xmlXPathNodeSetRemove` | `xml_xpath_node_set_remove` |
| `xpathInternals.h` | `xmlXPathNewNodeSetList` | `xml_xpath_new_node_set_list` |
| `xpathInternals.h` | `xmlXPathWrapNodeSet` | `xml_xpath_wrap_node_set` |
| `xpathInternals.h` | `xmlXPathWrapExternal` | `xml_xpath_wrap_external` |
| `xpathInternals.h` | `xmlXPathEqualValues` | `xml_xpath_equal_values` |
| `xpathInternals.h` | `xmlXPathNotEqualValues` | `xml_xpath_not_equal_values` |
| `xpathInternals.h` | `xmlXPathCompareValues` | `xml_xpath_compare_values` |
| `xpathInternals.h` | `xmlXPathValueFlipSign` | `xml_xpath_value_flip_sign` |
| `xpathInternals.h` | `xmlXPathAddValues` | `xml_xpath_add_values` |
| `xpathInternals.h` | `xmlXPathSubValues` | `xml_xpath_sub_values` |
| `xpathInternals.h` | `xmlXPathMultValues` | `xml_xpath_mult_values` |
| `xpathInternals.h` | `xmlXPathDivValues` | `xml_xpath_div_values` |
| `xpathInternals.h` | `xmlXPathModValues` | `xml_xpath_mod_values` |
| `xpathInternals.h` | `xmlXPathIsNodeType` | `xml_xpath_is_node_type` |
| `xpathInternals.h` | `xmlXPathLastFunction` | `xml_xpath_last_function` |
| `xpathInternals.h` | `xmlXPathPositionFunction` | `xml_xpath_position_function` |
| `xpathInternals.h` | `xmlXPathCountFunction` | `xml_xpath_count_function` |
| `xpathInternals.h` | `xmlXPathIdFunction` | `xml_xpath_id_function` |
| `xpathInternals.h` | `xmlXPathLocalNameFunction` | `xml_xpath_local_name_function` |
| `xpathInternals.h` | `xmlXPathNamespaceURIFunction` | `xml_xpath_namespace_urifunction` |
| `xpathInternals.h` | `xmlXPathStringFunction` | `xml_xpath_string_function` |
| `xpathInternals.h` | `xmlXPathStringLengthFunction` | `xml_xpath_string_length_function` |
| `xpathInternals.h` | `xmlXPathConcatFunction` | `xml_xpath_concat_function` |
| `xpathInternals.h` | `xmlXPathContainsFunction` | `xml_xpath_contains_function` |
| `xpathInternals.h` | `xmlXPathStartsWithFunction` | `xml_xpath_starts_with_function` |
| `xpathInternals.h` | `xmlXPathSubstringFunction` | `xml_xpath_substring_function` |
| `xpathInternals.h` | `xmlXPathSubstringBeforeFunction` | `xml_xpath_substring_before_function` |
| `xpathInternals.h` | `xmlXPathSubstringAfterFunction` | `xml_xpath_substring_after_function` |
| `xpathInternals.h` | `xmlXPathNormalizeFunction` | `xml_xpath_normalize_function` |
| `xpathInternals.h` | `xmlXPathTranslateFunction` | `xml_xpath_translate_function` |
| `xpathInternals.h` | `xmlXPathNotFunction` | `xml_xpath_not_function` |
| `xpathInternals.h` | `xmlXPathTrueFunction` | `xml_xpath_true_function` |
| `xpathInternals.h` | `xmlXPathFalseFunction` | `xml_xpath_false_function` |
| `xpathInternals.h` | `xmlXPathLangFunction` | `xml_xpath_lang_function` |
| `xpathInternals.h` | `xmlXPathNumberFunction` | `xml_xpath_number_function` |
| `xpathInternals.h` | `xmlXPathSumFunction` | `xml_xpath_sum_function` |
| `xpathInternals.h` | `xmlXPathFloorFunction` | `xml_xpath_floor_function` |
| `xpathInternals.h` | `xmlXPathCeilingFunction` | `xml_xpath_ceiling_function` |
| `xpathInternals.h` | `xmlXPathRoundFunction` | `xml_xpath_round_function` |
| `xpathInternals.h` | `xmlXPathBooleanFunction` | `xml_xpath_boolean_function` |
| `xpathInternals.h` | `xmlXPathNodeSetFreeNs` | `xml_xpath_node_set_free_ns` |
| `xpointer.h` | `xmlXPtrEval` | `xml_xptr_eval` |

## Deprecated

| header | libxml2 | maps toward |
|---|---|---|
| `catalog.h` | `xmlNewCatalog` | `xml_new_catalog` (not in facade) |
| `catalog.h` | `xmlLoadACatalog` | `xml_load_acatalog` (not in facade) |
| `catalog.h` | `xmlLoadSGMLSuperCatalog` | `xml_load_sgmlsuper_catalog` (not in facade) |
| `catalog.h` | `xmlConvertSGMLCatalog` | `xml_convert_sgmlcatalog` (not in facade) |
| `catalog.h` | `xmlACatalogAdd` | `xml_acatalog_add` (not in facade) |
| `catalog.h` | `xmlACatalogRemove` | `xml_acatalog_remove` (not in facade) |
| `catalog.h` | `xmlACatalogResolve` | `xml_acatalog_resolve` (not in facade) |
| `catalog.h` | `xmlACatalogResolveSystem` | `xml_acatalog_resolve_system` (not in facade) |
| `catalog.h` | `xmlACatalogResolvePublic` | `xml_acatalog_resolve_public` (not in facade) |
| `catalog.h` | `xmlACatalogResolveURI` | `xml_acatalog_resolve_uri` (not in facade) |
| `catalog.h` | `xmlACatalogDump` | `xml_acatalog_dump` (not in facade) |
| `catalog.h` | `xmlFreeCatalog` | `xml_free_catalog` (not in facade) |
| `catalog.h` | `xmlCatalogIsEmpty` | `xml_catalog_is_empty` (not in facade) |
| `catalog.h` | `xmlCatalogRemove` | `xml_catalog_remove` (not in facade) |
| `catalog.h` | `xmlParseCatalogFile` | `xml_parse_catalog_file` (not in facade) |
| `catalog.h` | `xmlCatalogConvert` | `xml_catalog_convert` (not in facade) |
| `catalog.h` | `xmlCatalogSetDebug` | `xml_catalog_set_debug` (not in facade) |
| `catalog.h` | `xmlCatalogSetDefaultPrefer` | `xml_catalog_set_default_prefer` (not in facade) |
| `catalog.h` | `xmlCatalogGetSystem` | `xml_catalog_get_system` (not in facade) |
| `catalog.h` | `xmlCatalogGetPublic` | `xml_catalog_get_public` (not in facade) |
| `chvalid.h` | `xmlIsBaseChar` | `xml_is_base_char` (not in facade) |
| `chvalid.h` | `xmlIsBlank` | `xml_is_blank` (not in facade) |
| `chvalid.h` | `xmlIsChar` | `xml_is_char` (not in facade) |
| `chvalid.h` | `xmlIsCombining` | `xml_is_combining` (not in facade) |
| `chvalid.h` | `xmlIsDigit` | `xml_is_digit` (not in facade) |
| `chvalid.h` | `xmlIsExtender` | `xml_is_extender` (not in facade) |
| `chvalid.h` | `xmlIsIdeographic` | `xml_is_ideographic` (not in facade) |
| `chvalid.h` | `xmlIsPubidChar` | `xml_is_pubid_char` (not in facade) |
| `dict.h` | `xmlInitializeDict` | `xml_initialize_dict` (not in facade) |
| `dict.h` | `xmlDictCleanup` | `xml_dict_cleanup` (not in facade) |
| `encoding.h` | `xmlInitCharEncodingHandlers` | `xml_init_char_encoding_handlers` (not in facade) |
| `encoding.h` | `xmlCleanupCharEncodingHandlers` | `xml_cleanup_char_encoding_handlers` (not in facade) |
| `encoding.h` | `xmlRegisterCharEncodingHandler` | `xml_register_char_encoding_handler` (not in facade) |
| `encoding.h` | `xmlGetCharEncodingHandler` | `xml_get_char_encoding_handler` (not in facade) |
| `encoding.h` | `xmlFindCharEncodingHandler` | `xml_find_char_encoding_handler` (not in facade) |
| `encoding.h` | `xmlNewCharEncodingHandler` | `xml_new_char_encoding_handler` (not in facade) |
| `encoding.h` | `xmlAddEncodingAlias` | `xml_add_encoding_alias` (not in facade) |
| `encoding.h` | `xmlDelEncodingAlias` | `xml_del_encoding_alias` (not in facade) |
| `encoding.h` | `xmlGetEncodingAlias` | `xml_get_encoding_alias` (not in facade) |
| `encoding.h` | `xmlCleanupEncodingAliases` | `xml_cleanup_encoding_aliases` (not in facade) |
| `encoding.h` | `xmlCharEncInFunc` | `xml_char_enc_in_func` (not in facade) |
| `encoding.h` | `xmlCharEncFirstLine` | `xml_char_enc_first_line` (not in facade) |
| `entities.h` | `xmlEncodeSpecialChars` | `xml_encode_special_chars` (not in facade) |
| `entities.h` | `xmlCreateEntitiesTable` | `xml_create_entities_table` (not in facade) |
| `entities.h` | `xmlCopyEntitiesTable` | `xml_copy_entities_table` (not in facade) |
| `entities.h` | `xmlFreeEntitiesTable` | `xml_free_entities_table` (not in facade) |
| `entities.h` | `xmlDumpEntitiesTable` | `xml_dump_entities_table` (not in facade) |
| `entities.h` | `xmlDumpEntityDecl` | `xml_dump_entity_decl` (not in facade) |
| `HTMLparser.h` | `htmlInitAutoClose` | `html_init_auto_close` (not in facade) |
| `HTMLparser.h` | `htmlTagLookup` | `html_tag_lookup` (not in facade) |
| `HTMLparser.h` | `htmlEntityLookup` | `html_entity_lookup` (not in facade) |
| `HTMLparser.h` | `htmlEntityValueLookup` | `html_entity_value_lookup` (not in facade) |
| `HTMLparser.h` | `htmlIsAutoClosed` | `html_is_auto_closed` (not in facade) |
| `HTMLparser.h` | `htmlAutoCloseTag` | `html_auto_close_tag` (not in facade) |
| `HTMLparser.h` | `htmlParseEntityRef` | `html_parse_entity_ref` (not in facade) |
| `HTMLparser.h` | `htmlParseCharRef` | `html_parse_char_ref` (not in facade) |
| `HTMLparser.h` | `htmlParseElement` | `html_parse_element` (not in facade) |
| `HTMLparser.h` | `htmlParseDocument` | `html_parse_document` (not in facade) |
| `HTMLparser.h` | `htmlSAXParseDoc` | `html_saxparse_doc` (not in facade) |
| `HTMLparser.h` | `htmlCreateFileParserCtxt` | `html_create_file_parser_ctxt` (not in facade) |
| `HTMLparser.h` | `htmlSAXParseFile` | `html_saxparse_file` (not in facade) |
| `HTMLparser.h` | `htmlParseFile` | `html_parse_file` (not in facade) |
| `HTMLparser.h` | `htmlUTF8ToHtml` | `html_utf8_to_html` (not in facade) |
| `HTMLparser.h` | `htmlEncodeEntities` | `html_encode_entities` (not in facade) |
| `HTMLparser.h` | `htmlIsScriptAttribute` | `html_is_script_attribute` (not in facade) |
| `HTMLparser.h` | `htmlHandleOmittedElem` | `html_handle_omitted_elem` (not in facade) |
| `HTMLparser.h` | `htmlAttrAllowed` | `html_attr_allowed` (not in facade) |
| `HTMLparser.h` | `htmlElementAllowedHere` | `html_element_allowed_here` (not in facade) |
| `HTMLparser.h` | `htmlElementStatusHere` | `html_element_status_here` (not in facade) |
| `HTMLparser.h` | `htmlNodeStatus` | `html_node_status` (not in facade) |
| `HTMLtree.h` | `htmlIsBooleanAttr` | `html_is_boolean_attr` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPInit` | `xml_nano_httpinit` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPCleanup` | `xml_nano_httpcleanup` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPScanProxy` | `xml_nano_httpscan_proxy` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPFetch` | `xml_nano_httpfetch` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPMethod` | `xml_nano_httpmethod` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPMethodRedir` | `xml_nano_httpmethod_redir` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPOpen` | `xml_nano_httpopen` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPOpenRedir` | `xml_nano_httpopen_redir` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPReturnCode` | `xml_nano_httpreturn_code` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPAuthHeader` | `xml_nano_httpauth_header` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPRedir` | `xml_nano_httpredir` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPContentLength` | `xml_nano_httpcontent_length` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPEncoding` | `xml_nano_httpencoding` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPMimeType` | `xml_nano_httpmime_type` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPRead` | `xml_nano_httpread` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPSave` | `xml_nano_httpsave` (not in facade) |
| `nanohttp.h` | `xmlNanoHTTPClose` | `xml_nano_httpclose` (not in facade) |
| `parser.h` | `xmlInitParser` | `xml_init_parser` (not in facade) |
| `parser.h` | `xmlCleanupParser` | `xml_cleanup_parser` (not in facade) |
| `parser.h` | `xmlInitGlobals` | `xml_init_globals` (not in facade) |
| `parser.h` | `xmlCleanupGlobals` | `xml_cleanup_globals` (not in facade) |
| `parser.h` | `xmlParserInputRead` | `xml_parser_input_read` (not in facade) |
| `parser.h` | `xmlParseMemory` | `xml_parse_memory` (not in facade) |
| `parser.h` | `xmlSubstituteEntitiesDefault` | `xml_substitute_entities_default` (not in facade) |
| `parser.h` | `xmlKeepBlanksDefault` | `xml_keep_blanks_default` (not in facade) |
| `parser.h` | `xmlStopParser` | `xml_stop_parser` (not in facade) |
| `parser.h` | `xmlPedanticParserDefault` | `xml_pedantic_parser_default` (not in facade) |
| `parser.h` | `xmlLineNumbersDefault` | `xml_line_numbers_default` (not in facade) |
| `parser.h` | `xmlThrDefSubstituteEntitiesDefaultValue` | `xml_thr_def_substitute_entities_default_value` (not in facade) |
| `parser.h` | `xmlThrDefKeepBlanksDefaultValue` | `xml_thr_def_keep_blanks_default_value` (not in facade) |
| `parser.h` | `xmlThrDefPedanticParserDefaultValue` | `xml_thr_def_pedantic_parser_default_value` (not in facade) |
| `parser.h` | `xmlThrDefLineNumbersDefaultValue` | `xml_thr_def_line_numbers_default_value` (not in facade) |
| `parser.h` | `xmlThrDefDoValidityCheckingDefaultValue` | `xml_thr_def_do_validity_checking_default_value` (not in facade) |
| `parser.h` | `xmlThrDefGetWarningsDefaultValue` | `xml_thr_def_get_warnings_default_value` (not in facade) |
| `parser.h` | `xmlThrDefLoadExtDtdDefaultValue` | `xml_thr_def_load_ext_dtd_default_value` (not in facade) |
| `parser.h` | `xmlRecoverDoc` | `xml_recover_doc` (not in facade) |
| `parser.h` | `xmlRecoverMemory` | `xml_recover_memory` (not in facade) |
| `parser.h` | `xmlRecoverFile` | `xml_recover_file` (not in facade) |
| `parser.h` | `xmlParseDocument` | `xml_parse_document` (not in facade) |
| `parser.h` | `xmlParseExtParsedEnt` | `xml_parse_ext_parsed_ent` (not in facade) |
| `parser.h` | `xmlSAXUserParseFile` | `xml_sax_user_parse_file` (not in facade) |
| `parser.h` | `xmlSAXUserParseMemory` | `xml_sax_user_parse_memory` (not in facade) |
| `parser.h` | `xmlSAXParseDoc` | `xml_sax_parse_doc` (not in facade) |
| `parser.h` | `xmlSAXParseMemory` | `xml_sax_parse_memory` (not in facade) |
| `parser.h` | `xmlSAXParseMemoryWithData` | `xml_sax_parse_memory_with_data` (not in facade) |
| `parser.h` | `xmlSAXParseFile` | `xml_sax_parse_file` (not in facade) |
| `parser.h` | `xmlSAXParseFileWithData` | `xml_sax_parse_file_with_data` (not in facade) |
| `parser.h` | `xmlSAXParseEntity` | `xml_sax_parse_entity` (not in facade) |
| `parser.h` | `xmlParseEntity` | `xml_parse_entity` (not in facade) |
| `parser.h` | `xmlCtxtValidateDtd` | `xml_ctxt_validate_dtd` (not in facade) |
| `parser.h` | `xmlSAXParseDTD` | `xml_sax_parse_dtd` (not in facade) |
| `parser.h` | `xmlParseExternalEntity` | `xml_parse_external_entity` (not in facade) |
| `parser.h` | `xmlInitParserCtxt` | `xml_init_parser_ctxt` (not in facade) |
| `parser.h` | `xmlClearParserCtxt` | `xml_clear_parser_ctxt` (not in facade) |
| `parser.h` | `xmlFreeParserCtxt` | `xml_free_parser_ctxt` (not in facade) |
| `parser.h` | `xmlSetupParserForBuffer` | `xml_setup_parser_for_buffer` (not in facade) |
| `parser.h` | `xmlParserFindNodeInfo` | `xml_parser_find_node_info` (not in facade) |
| `parser.h` | `xmlInitNodeInfoSeq` | `xml_init_node_info_seq` (not in facade) |
| `parser.h` | `xmlClearNodeInfoSeq` | `xml_clear_node_info_seq` (not in facade) |
| `parser.h` | `xmlParserFindNodeInfoIndex` | `xml_parser_find_node_info_index` (not in facade) |
| `parser.h` | `xmlParserAddNodeInfo` | `xml_parser_add_node_info` (not in facade) |
| `parser.h` | `xmlByteConsumed` | `xml_byte_consumed` (not in facade) |
| `parserInternals.h` | `xmlIsLetter` | `xml_is_letter` (not in facade) |
| `parserInternals.h` | `xmlCreateMemoryParserCtxt` | `xml_create_memory_parser_ctxt` (not in facade) |
| `parserInternals.h` | `xmlCreateEntityParserCtxt` | `xml_create_entity_parser_ctxt` (not in facade) |
| `parserInternals.h` | `xmlSwitchToEncoding` | `xml_switch_to_encoding` (not in facade) |
| `parserInternals.h` | `xmlSwitchInputEncoding` | `xml_switch_input_encoding` (not in facade) |
| `parserInternals.h` | `xmlNewStringInputStream` | `xml_new_string_input_stream` (not in facade) |
| `parserInternals.h` | `xmlNewEntityInputStream` | `xml_new_entity_input_stream` (not in facade) |
| `parserInternals.h` | `xmlCtxtPopInput` | `xml_ctxt_pop_input` (not in facade) |
| `parserInternals.h` | `xmlPushInput` | `xml_push_input` (not in facade) |
| `parserInternals.h` | `xmlPopInput` | `xml_pop_input` (not in facade) |
| `parserInternals.h` | `xmlParseName` | `xml_parse_name` (not in facade) |
| `parserInternals.h` | `xmlParseNmtoken` | `xml_parse_nmtoken` (not in facade) |
| `parserInternals.h` | `xmlParseEntityValue` | `xml_parse_entity_value` (not in facade) |
| `parserInternals.h` | `xmlParseAttValue` | `xml_parse_att_value` (not in facade) |
| `parserInternals.h` | `xmlParseSystemLiteral` | `xml_parse_system_literal` (not in facade) |
| `parserInternals.h` | `xmlParsePubidLiteral` | `xml_parse_pubid_literal` (not in facade) |
| `parserInternals.h` | `xmlParseCharData` | `xml_parse_char_data` (not in facade) |
| `parserInternals.h` | `xmlParseExternalID` | `xml_parse_external_id` (not in facade) |
| `parserInternals.h` | `xmlParseComment` | `xml_parse_comment` (not in facade) |
| `parserInternals.h` | `xmlParsePITarget` | `xml_parse_pitarget` (not in facade) |
| `parserInternals.h` | `xmlParsePI` | `xml_parse_pi` (not in facade) |
| `parserInternals.h` | `xmlParseNotationDecl` | `xml_parse_notation_decl` (not in facade) |
| `parserInternals.h` | `xmlParseEntityDecl` | `xml_parse_entity_decl` (not in facade) |
| `parserInternals.h` | `xmlParseDefaultDecl` | `xml_parse_default_decl` (not in facade) |
| `parserInternals.h` | `xmlParseNotationType` | `xml_parse_notation_type` (not in facade) |
| `parserInternals.h` | `xmlParseEnumerationType` | `xml_parse_enumeration_type` (not in facade) |
| `parserInternals.h` | `xmlParseEnumeratedType` | `xml_parse_enumerated_type` (not in facade) |
| `parserInternals.h` | `xmlParseAttributeType` | `xml_parse_attribute_type` (not in facade) |
| `parserInternals.h` | `xmlParseAttributeListDecl` | `xml_parse_attribute_list_decl` (not in facade) |
| `parserInternals.h` | `xmlParseElementMixedContentDecl` | `xml_parse_element_mixed_content_decl` (not in facade) |
| `parserInternals.h` | `xmlParseElementChildrenContentDecl` | `xml_parse_element_children_content_decl` (not in facade) |
| `parserInternals.h` | `xmlParseElementContentDecl` | `xml_parse_element_content_decl` (not in facade) |
| `parserInternals.h` | `xmlParseElementDecl` | `xml_parse_element_decl` (not in facade) |
| `parserInternals.h` | `xmlParseMarkupDecl` | `xml_parse_markup_decl` (not in facade) |
| `parserInternals.h` | `xmlParseCharRef` | `xml_parse_char_ref` (not in facade) |
| `parserInternals.h` | `xmlParseEntityRef` | `xml_parse_entity_ref` (not in facade) |
| `parserInternals.h` | `xmlParseReference` | `xml_parse_reference` (not in facade) |
| `parserInternals.h` | `xmlParsePEReference` | `xml_parse_pereference` (not in facade) |
| `parserInternals.h` | `xmlParseDocTypeDecl` | `xml_parse_doc_type_decl` (not in facade) |
| `parserInternals.h` | `xmlParseAttribute` | `xml_parse_attribute` (not in facade) |
| `parserInternals.h` | `xmlParseStartTag` | `xml_parse_start_tag` (not in facade) |
| `parserInternals.h` | `xmlParseEndTag` | `xml_parse_end_tag` (not in facade) |
| `parserInternals.h` | `xmlParseCDSect` | `xml_parse_cdsect` (not in facade) |
| `parserInternals.h` | `xmlParseContent` | `xml_parse_content` (not in facade) |
| `parserInternals.h` | `xmlParseElement` | `xml_parse_element` (not in facade) |
| `parserInternals.h` | `xmlParseVersionNum` | `xml_parse_version_num` (not in facade) |
| `parserInternals.h` | `xmlParseVersionInfo` | `xml_parse_version_info` (not in facade) |
| `parserInternals.h` | `xmlParseEncName` | `xml_parse_enc_name` (not in facade) |
| `parserInternals.h` | `xmlParseEncodingDecl` | `xml_parse_encoding_decl` (not in facade) |
| `parserInternals.h` | `xmlParseSDDecl` | `xml_parse_sddecl` (not in facade) |
| `parserInternals.h` | `xmlParseXMLDecl` | `xml_parse_xmldecl` (not in facade) |
| `parserInternals.h` | `xmlParseTextDecl` | `xml_parse_text_decl` (not in facade) |
| `parserInternals.h` | `xmlParseMisc` | `xml_parse_misc` (not in facade) |
| `parserInternals.h` | `xmlParseExternalSubset` | `xml_parse_external_subset` (not in facade) |
| `parserInternals.h` | `xmlStringDecodeEntities` | `xml_string_decode_entities` (not in facade) |
| `parserInternals.h` | `xmlStringLenDecodeEntities` | `xml_string_len_decode_entities` (not in facade) |
| `parserInternals.h` | `xmlSkipBlankChars` | `xml_skip_blank_chars` (not in facade) |
| `parserInternals.h` | `xmlStringCurrentChar` | `xml_string_current_char` (not in facade) |
| `parserInternals.h` | `xmlParserHandlePEReference` | `xml_parser_handle_pereference` (not in facade) |
| `parserInternals.h` | `xmlCheckLanguageID` | `xml_check_language_id` (not in facade) |
| `parserInternals.h` | `xmlCurrentChar` | `xml_current_char` (not in facade) |
| `parserInternals.h` | `xmlCopyCharMultiByte` | `xml_copy_char_multi_byte` (not in facade) |
| `parserInternals.h` | `xmlCopyChar` | `xml_copy_char` (not in facade) |
| `parserInternals.h` | `xmlNextChar` | `xml_next_char` (not in facade) |
| `parserInternals.h` | `xmlParserInputShrink` | `xml_parser_input_shrink` (not in facade) |
| `relaxng.h` | `xmlRelaxNGInitTypes` | `xml_relax_nginit_types` (not in facade) |
| `relaxng.h` | `xmlRelaxNGCleanupTypes` | `xml_relax_ngcleanup_types` (not in facade) |
| `SAX2.h` | `xmlSAX2StartDocument` | `xml_sax2_start_document` (not in facade) |
| `SAX2.h` | `xmlSAX2EndDocument` | `xml_sax2_end_document` (not in facade) |
| `SAX2.h` | `xmlSAX2StartElement` | `xml_sax2_start_element` (not in facade) |
| `SAX2.h` | `xmlSAX2EndElement` | `xml_sax2_end_element` (not in facade) |
| `SAX2.h` | `xmlSAXDefaultVersion` | `xml_sax_default_version` (not in facade) |
| `SAX2.h` | `xmlSAX2InitHtmlDefaultSAXHandler` | `xml_sax2_init_html_default_saxhandler` (not in facade) |
| `SAX2.h` | `htmlDefaultSAXHandlerInit` | `html_default_saxhandler_init` (not in facade) |
| `SAX2.h` | `xmlDefaultSAXHandlerInit` | `xml_default_saxhandler_init` (not in facade) |
| `threads.h` | `xmlInitThreads` | `xml_init_threads` (not in facade) |
| `threads.h` | `xmlLockLibrary` | `xml_lock_library` (not in facade) |
| `threads.h` | `xmlUnlockLibrary` | `xml_unlock_library` (not in facade) |
| `threads.h` | `xmlCleanupThreads` | `xml_cleanup_threads` (not in facade) |
| `tree.h` | `xmlSetDocCompressMode` | `xml_set_doc_compress_mode` (not in facade) |
| `tree.h` | `xmlGetCompressMode` | `xml_get_compress_mode` (not in facade) |
| `tree.h` | `xmlSetCompressMode` | `xml_set_compress_mode` (not in facade) |
| `tree.h` | `xmlPreviousElementSibling` | `xml_previous_element_sibling` (not in facade) |
| `tree.h` | `xmlRegisterNodeDefault` | `xml_register_node_default` (not in facade) |
| `tree.h` | `xmlDeregisterNodeDefault` | `xml_deregister_node_default` (not in facade) |
| `tree.h` | `xmlThrDefRegisterNodeDefault` | `xml_thr_def_register_node_default` (not in facade) |
| `tree.h` | `xmlThrDefDeregisterNodeDefault` | `xml_thr_def_deregister_node_default` (not in facade) |
| `tree.h` | `xmlSetBufferAllocationScheme` | `xml_set_buffer_allocation_scheme` (not in facade) |
| `tree.h` | `xmlGetBufferAllocationScheme` | `xml_get_buffer_allocation_scheme` (not in facade) |
| `tree.h` | `xmlBufferCreateStatic` | `xml_buffer_create_static` (not in facade) |
| `tree.h` | `xmlBufferResize` | `xml_buffer_resize` (not in facade) |
| `tree.h` | `xmlBufferCCat` | `xml_buffer_ccat` (not in facade) |
| `tree.h` | `xmlBufferShrink` | `xml_buffer_shrink` (not in facade) |
| `tree.h` | `xmlBufferGrow` | `xml_buffer_grow` (not in facade) |
| `valid.h` | `xmlCopyNotationTable` | `xml_copy_notation_table` (not in facade) |
| `valid.h` | `xmlFreeNotationTable` | `xml_free_notation_table` (not in facade) |
| `valid.h` | `xmlDumpNotationDecl` | `xml_dump_notation_decl` (not in facade) |
| `valid.h` | `xmlDumpNotationTable` | `xml_dump_notation_table` (not in facade) |
| `valid.h` | `xmlNewElementContent` | `xml_new_element_content` (not in facade) |
| `valid.h` | `xmlCopyElementContent` | `xml_copy_element_content` (not in facade) |
| `valid.h` | `xmlFreeElementContent` | `xml_free_element_content` (not in facade) |
| `valid.h` | `xmlNewDocElementContent` | `xml_new_doc_element_content` (not in facade) |
| `valid.h` | `xmlCopyDocElementContent` | `xml_copy_doc_element_content` (not in facade) |
| `valid.h` | `xmlFreeDocElementContent` | `xml_free_doc_element_content` (not in facade) |
| `valid.h` | `xmlSnprintfElementContent` | `xml_snprintf_element_content` (not in facade) |
| `valid.h` | `xmlSprintfElementContent` | `xml_sprintf_element_content` (not in facade) |
| `valid.h` | `xmlCopyElementTable` | `xml_copy_element_table` (not in facade) |
| `valid.h` | `xmlFreeElementTable` | `xml_free_element_table` (not in facade) |
| `valid.h` | `xmlDumpElementTable` | `xml_dump_element_table` (not in facade) |
| `valid.h` | `xmlDumpElementDecl` | `xml_dump_element_decl` (not in facade) |
| `valid.h` | `xmlCreateEnumeration` | `xml_create_enumeration` (not in facade) |
| `valid.h` | `xmlFreeEnumeration` | `xml_free_enumeration` (not in facade) |
| `valid.h` | `xmlCopyEnumeration` | `xml_copy_enumeration` (not in facade) |
| `valid.h` | `xmlCopyAttributeTable` | `xml_copy_attribute_table` (not in facade) |
| `valid.h` | `xmlFreeAttributeTable` | `xml_free_attribute_table` (not in facade) |
| `valid.h` | `xmlDumpAttributeTable` | `xml_dump_attribute_table` (not in facade) |
| `valid.h` | `xmlDumpAttributeDecl` | `xml_dump_attribute_decl` (not in facade) |
| `valid.h` | `xmlAddRef` | `xml_add_ref` (not in facade) |
| `valid.h` | `xmlFreeRefTable` | `xml_free_ref_table` (not in facade) |
| `valid.h` | `xmlIsRef` | `xml_is_ref` (not in facade) |
| `valid.h` | `xmlRemoveRef` | `xml_remove_ref` (not in facade) |
| `valid.h` | `xmlGetRefs` | `xml_get_refs` (not in facade) |
| `valid.h` | `xmlFreeValidCtxt` | `xml_free_valid_ctxt` (not in facade) |
| `valid.h` | `xmlValidateRoot` | `xml_validate_root` (not in facade) |
| `valid.h` | `xmlValidateElementDecl` | `xml_validate_element_decl` (not in facade) |
| `valid.h` | `xmlValidNormalizeAttributeValue` | `xml_valid_normalize_attribute_value` (not in facade) |
| `valid.h` | `xmlValidCtxtNormalizeAttributeValue` | `xml_valid_ctxt_normalize_attribute_value` (not in facade) |
| `valid.h` | `xmlValidateAttributeDecl` | `xml_validate_attribute_decl` (not in facade) |
| `valid.h` | `xmlValidateAttributeValue` | `xml_validate_attribute_value` (not in facade) |
| `valid.h` | `xmlValidateNotationDecl` | `xml_validate_notation_decl` (not in facade) |
| `valid.h` | `xmlValidateDtd` | `xml_validate_dtd` (not in facade) |
| `valid.h` | `xmlValidateDtdFinal` | `xml_validate_dtd_final` (not in facade) |
| `valid.h` | `xmlValidateElement` | `xml_validate_element` (not in facade) |
| `valid.h` | `xmlValidateOneElement` | `xml_validate_one_element` (not in facade) |
| `valid.h` | `xmlValidateOneAttribute` | `xml_validate_one_attribute` (not in facade) |
| `valid.h` | `xmlValidateOneNamespace` | `xml_validate_one_namespace` (not in facade) |
| `valid.h` | `xmlValidateDocumentFinal` | `xml_validate_document_final` (not in facade) |
| `valid.h` | `xmlValidateNotationUse` | `xml_validate_notation_use` (not in facade) |
| `valid.h` | `xmlIsMixedElement` | `xml_is_mixed_element` (not in facade) |
| `valid.h` | `xmlValidBuildContentModel` | `xml_valid_build_content_model` (not in facade) |
| `valid.h` | `xmlValidatePushElement` | `xml_validate_push_element` (not in facade) |
| `valid.h` | `xmlValidatePushCData` | `xml_validate_push_cdata` (not in facade) |
| `valid.h` | `xmlValidatePopElement` | `xml_validate_pop_element` (not in facade) |
| `xlink.h` | `xlinkGetDefaultDetect` | `xlink_get_default_detect` (not in facade) |
| `xlink.h` | `xlinkSetDefaultDetect` | `xlink_set_default_detect` (not in facade) |
| `xlink.h` | `xlinkGetDefaultHandler` | `xlink_get_default_handler` (not in facade) |
| `xlink.h` | `xlinkSetDefaultHandler` | `xlink_set_default_handler` (not in facade) |
| `xlink.h` | `xlinkIsLink` | `xlink_is_link` (not in facade) |
| `xmlautomata.h` | `xmlNewAutomata` | `xml_new_automata` (not in facade) |
| `xmlautomata.h` | `xmlFreeAutomata` | `xml_free_automata` (not in facade) |
| `xmlautomata.h` | `xmlAutomataGetInitState` | `xml_automata_get_init_state` (not in facade) |
| `xmlautomata.h` | `xmlAutomataSetFinalState` | `xml_automata_set_final_state` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewState` | `xml_automata_new_state` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewTransition` | `xml_automata_new_transition` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewTransition2` | `xml_automata_new_transition2` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewNegTrans` | `xml_automata_new_neg_trans` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewCountTrans` | `xml_automata_new_count_trans` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewCountTrans2` | `xml_automata_new_count_trans2` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewOnceTrans` | `xml_automata_new_once_trans` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewOnceTrans2` | `xml_automata_new_once_trans2` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewAllTrans` | `xml_automata_new_all_trans` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewEpsilon` | `xml_automata_new_epsilon` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewCountedTrans` | `xml_automata_new_counted_trans` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewCounterTrans` | `xml_automata_new_counter_trans` (not in facade) |
| `xmlautomata.h` | `xmlAutomataNewCounter` | `xml_automata_new_counter` (not in facade) |
| `xmlautomata.h` | `xmlAutomataCompile` | `xml_automata_compile` (not in facade) |
| `xmlautomata.h` | `xmlAutomataIsDeterminist` | `xml_automata_is_determinist` (not in facade) |
| `xmlerror.h` | `xmlSetStructuredErrorFunc` | `xml_set_structured_error_func` (not in facade) |
| `xmlerror.h` | `xmlThrDefSetGenericErrorFunc` | `xml_thr_def_set_generic_error_func` (not in facade) |
| `xmlerror.h` | `xmlThrDefSetStructuredErrorFunc` | `xml_thr_def_set_structured_error_func` (not in facade) |
| `xmlIO.h` | `__xmlParserInputBufferCreateFilenameValue` | `__xml_parser_input_buffer_create_filename_value` (not in facade) |
| `xmlIO.h` | `__xmlOutputBufferCreateFilenameValue` | `__xml_output_buffer_create_filename_value` (not in facade) |
| `xmlIO.h` | `xmlParserInputBufferCreateFilename` | `xml_parser_input_buffer_create_filename` (not in facade) |
| `xmlIO.h` | `xmlParserInputBufferCreateFile` | `xml_parser_input_buffer_create_file` (not in facade) |
| `xmlIO.h` | `xmlParserInputBufferRead` | `xml_parser_input_buffer_read` (not in facade) |
| `xmlIO.h` | `xmlParserInputBufferGrow` | `xml_parser_input_buffer_grow` (not in facade) |
| `xmlIO.h` | `xmlParserInputBufferPush` | `xml_parser_input_buffer_push` (not in facade) |
| `xmlIO.h` | `xmlCheckHTTPInput` | `xml_check_httpinput` (not in facade) |
| `xmlIO.h` | `xmlNoNetExternalEntityLoader` | `xml_no_net_external_entity_loader` (not in facade) |
| `xmlIO.h` | `xmlNormalizeWindowsPath` | `xml_normalize_windows_path` (not in facade) |
| `xmlIO.h` | `xmlCheckFilename` | `xml_check_filename` (not in facade) |
| `xmlIO.h` | `xmlFileMatch` | `xml_file_match` (not in facade) |
| `xmlIO.h` | `xmlFileOpen` | `xml_file_open` (not in facade) |
| `xmlIO.h` | `xmlFileRead` | `xml_file_read` (not in facade) |
| `xmlIO.h` | `xmlFileClose` | `xml_file_close` (not in facade) |
| `xmlIO.h` | `xmlIOHTTPMatch` | `xml_io_httpmatch` (not in facade) |
| `xmlIO.h` | `xmlIOHTTPOpen` | `xml_io_httpopen` (not in facade) |
| `xmlIO.h` | `xmlRegisterHTTPPostCallbacks` | `xml_register_httppost_callbacks` (not in facade) |
| `xmlIO.h` | `xmlIOHTTPOpenW` | `xml_io_httpopen_w` (not in facade) |
| `xmlIO.h` | `xmlIOHTTPRead` | `xml_io_httpread` (not in facade) |
| `xmlIO.h` | `xmlIOHTTPClose` | `xml_io_httpclose` (not in facade) |
| `xmlIO.h` | `xmlOutputBufferCreateFilenameDefault` | `xml_output_buffer_create_filename_default` (not in facade) |
| `xmlIO.h` | `xmlThrDefOutputBufferCreateFilenameDefault` | `xml_thr_def_output_buffer_create_filename_default` (not in facade) |
| `xmlIO.h` | `xmlThrDefParserInputBufferCreateFilenameDefault` | `xml_thr_def_parser_input_buffer_create_filename_default` (not in facade) |
| `xmlmemory.h` | `xmlGcMemSetup` | `xml_gc_mem_setup` (not in facade) |
| `xmlmemory.h` | `xmlGcMemGet` | `xml_gc_mem_get` (not in facade) |
| `xmlmemory.h` | `xmlInitMemory` | `xml_init_memory` (not in facade) |
| `xmlmemory.h` | `xmlCleanupMemory` | `xml_cleanup_memory` (not in facade) |
| `xmlmemory.h` | `xmlMemUsed` | `xml_mem_used` (not in facade) |
| `xmlmemory.h` | `xmlMemBlocks` | `xml_mem_blocks` (not in facade) |
| `xmlmemory.h` | `xmlMemDisplay` | `xml_mem_display` (not in facade) |
| `xmlmemory.h` | `xmlMemDisplayLast` | `xml_mem_display_last` (not in facade) |
| `xmlmemory.h` | `xmlMemShow` | `xml_mem_show` (not in facade) |
| `xmlmemory.h` | `xmlMemoryDump` | `xml_memory_dump` (not in facade) |
| `xmlmemory.h` | `xmlMemFree` | `xml_mem_free` (not in facade) |
| `xmlmemory.h` | `xmlMemoryStrdup` | `xml_memory_strdup` (not in facade) |
| `xmlmemory.h` | `xmlMallocLoc` | `xml_malloc_loc` (not in facade) |
| `xmlmemory.h` | `xmlReallocLoc` | `xml_realloc_loc` (not in facade) |
| `xmlmemory.h` | `xmlMallocAtomicLoc` | `xml_malloc_atomic_loc` (not in facade) |
| `xmlmemory.h` | `xmlMemStrdupLoc` | `xml_mem_strdup_loc` (not in facade) |
| `xmlmodule.h` | `xmlModuleSymbol` | `xml_module_symbol` (not in facade) |
| `xmlmodule.h` | `xmlModuleSymbol` | `xml_module_symbol` (not in facade) |
| `xmlmodule.h` | `xmlModuleClose` | `xml_module_close` (not in facade) |
| `xmlmodule.h` | `xmlModuleFree` | `xml_module_free` (not in facade) |
| `xmlregexp.h` | `xmlRegFreeRegexp` | `xml_reg_free_regexp` (not in facade) |
| `xmlregexp.h` | `xmlRegexpExec` | `xml_regexp_exec` (not in facade) |
| `xmlregexp.h` | `xmlRegexpPrint` | `xml_regexp_print` (not in facade) |
| `xmlregexp.h` | `xmlRegNewExecCtxt` | `xml_reg_new_exec_ctxt` (not in facade) |
| `xmlregexp.h` | `xmlRegFreeExecCtxt` | `xml_reg_free_exec_ctxt` (not in facade) |
| `xmlregexp.h` | `xmlRegExecPushString` | `xml_reg_exec_push_string` (not in facade) |
| `xmlregexp.h` | `xmlRegExecPushString2` | `xml_reg_exec_push_string2` (not in facade) |
| `xmlregexp.h` | `xmlRegExecNextValues` | `xml_reg_exec_next_values` (not in facade) |
| `xmlregexp.h` | `xmlRegExecErrInfo` | `xml_reg_exec_err_info` (not in facade) |
| `xmlsave.h` | `xmlSaveSetIndentString` | `xml_save_set_indent_string` (not in facade) |
| `xmlsave.h` | `xmlSaveSetEscape` | `xml_save_set_escape` (not in facade) |
| `xmlsave.h` | `xmlSaveSetAttrEscape` | `xml_save_set_attr_escape` (not in facade) |
| `xmlsave.h` | `xmlThrDefIndentTreeOutput` | `xml_thr_def_indent_tree_output` (not in facade) |
| `xmlsave.h` | `xmlThrDefTreeIndentString` | `xml_thr_def_tree_indent_string` (not in facade) |
| `xmlsave.h` | `xmlThrDefSaveNoEmptyTags` | `xml_thr_def_save_no_empty_tags` (not in facade) |
| `xmlschemastypes.h` | `xmlSchemaInitTypes` | `xml_schema_init_types` (not in facade) |
| `xmlschemastypes.h` | `xmlSchemaCleanupTypes` | `xml_schema_cleanup_types` (not in facade) |
| `xpath.h` | `xmlXPathFreeCompExpr` | `xml_xpath_free_comp_expr` (not in facade) |
| `xpath.h` | `xmlXPathInit` | `xml_xpath_init` (not in facade) |
| `xpathInternals.h` | `xmlXPathRoot` | `xml_xpath_root` (not in facade) |
| `xpathInternals.h` | `xmlXPathEvalExpr` | `xml_xpath_eval_expr` (not in facade) |
| `xpointer.h` | `xmlXPtrNewContext` | `xml_xptr_new_context` (not in facade) |
