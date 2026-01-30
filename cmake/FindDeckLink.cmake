# FindDeckLink.cmake - Locate Blackmagic DeckLink SDK

if(APPLE)
    # macOS paths
    set(DECKLINK_SEARCH_PATHS
        "${CMAKE_SOURCE_DIR}/libs/decklink-sdk"
        "/Library/Application Support/Blackmagic Design/Blackmagic DeckLink"
        "$ENV{HOME}/Library/Application Support/Blackmagic Design/Blackmagic DeckLink"
    )
    
    find_path(DECKLINK_INCLUDE_DIR
        NAMES DeckLinkAPI.h
        PATHS ${DECKLINK_SEARCH_PATHS}
        PATH_SUFFIXES include Mac/include
    )
    
    # On macOS, DeckLink is header-only with system frameworks
    if(DECKLINK_INCLUDE_DIR)
        set(DECKLINK_FOUND TRUE)
        set(DECKLINK_INCLUDE_DIRS ${DECKLINK_INCLUDE_DIR})
        set(DECKLINK_LIBRARIES "")
        
        # Add required macOS frameworks
        find_library(COREFOUNDATION CoreFoundation)
        find_library(COREVIDEO CoreVideo)
        list(APPEND DECKLINK_LIBRARIES ${COREFOUNDATION} ${COREVIDEO})
    endif()
    
elseif(WIN32)
    # Windows paths
    set(DECKLINK_SEARCH_PATHS
        "${CMAKE_SOURCE_DIR}/libs/decklink-sdk"
        "C:/Program Files/Blackmagic Design/DeckLink"
        "$ENV{PROGRAMFILES}/Blackmagic Design/DeckLink"
    )
    
    find_path(DECKLINK_INCLUDE_DIR
        NAMES DeckLinkAPI.h
        PATHS ${DECKLINK_SEARCH_PATHS}
        PATH_SUFFIXES include Win/include
    )
    
    find_library(DECKLINK_LIBRARY
        NAMES DeckLinkAPI
        PATHS ${DECKLINK_SEARCH_PATHS}
        PATH_SUFFIXES lib Win/lib
    )
    
    if(DECKLINK_INCLUDE_DIR AND DECKLINK_LIBRARY)
        set(DECKLINK_FOUND TRUE)
        set(DECKLINK_INCLUDE_DIRS ${DECKLINK_INCLUDE_DIR})
        set(DECKLINK_LIBRARIES ${DECKLINK_LIBRARY})
    endif()
endif()

include(FindPackageHandleStandardArgs)
find_package_handle_standard_args(DeckLink
    REQUIRED_VARS DECKLINK_INCLUDE_DIR
    VERSION_VAR DECKLINK_VERSION
)

mark_as_advanced(DECKLINK_INCLUDE_DIR DECKLINK_LIBRARY)

if(DECKLINK_FOUND AND NOT TARGET DeckLink::DeckLink)
    add_library(DeckLink::DeckLink INTERFACE IMPORTED)
    set_target_properties(DeckLink::DeckLink PROPERTIES
        INTERFACE_INCLUDE_DIRECTORIES "${DECKLINK_INCLUDE_DIRS}"
        INTERFACE_LINK_LIBRARIES "${DECKLINK_LIBRARIES}"
    )
endif()
