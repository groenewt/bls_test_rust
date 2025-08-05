CENSUS OF FATAL OCCUPATIONAL INJURIES (FW)
				fw.txt

Section Listing

1. Survey Definition
2. FTP files listed in the survey directory.
3. Time series, series file, data file, & mapping file definitions and    
   relationships
4. Series file format and field definitions
5. Data file format and field definitions
6. Mapping file formats and field definitions
7. Data Element Dictionary

================================================================================
Section 1
================================================================================

The following is a definition of: CENSUS OF FATAL OCCUPATIONAL INJURIES (FW)

Survey Description:   

The Bureau of Labor Statistics (BLS) Census of Fatal Occupational Injuries (CFOI) produces 
comprehensive, accurate, and timely counts of fatal work injuries.  CFOI is a Federal-State 
cooperative program that has been active in all 50 States and the District of Columbia since 1992. 
To compile counts that are as complete as possible, CFOI uses multiple sources to identify, 
verify, and profile fatal worker injuries.  Information about each workplace fatal injury is 
obtained by cross referencing the source records, such as death certificates, workers' 
compensation reports, and Federal and State agency administrative reports.  To ensure that fatal 
injuries are work-related, cases are substantiated with two or more independent source documents, 
or a source document and a follow-up questionnaire.

CFOI publishes detailed data on case characteristics such as the event or exposure leading to the 
fatal injury, the source of the fatal injury, and worker activity at the time of the incident.  
Event or exposure, source, and other case characteristic codes are currently based on the BLS 
Occupational Injury and Illness Classification System (OIICS) 2.01, which can be viewed at our Web 
site at http://www.bls.gov/iif/oshoiics.htm.

BLS also tabulates the number of fatal occupational injuries by demographic attributes such as 
age, gender, race, ethnic origin, and occupation.  Occupation classifications are currently based 
on the 2010 Standard Occupational Classification [SOC] system.  These tabulations are also 
available by major industry sectors or by detailed industry. Industry classifications are currently 
based on the 2007 North American Industrial Classification System [NAICS].

Frequency of observations:  All data are annual.

Data characteristics:  All figures are actual numbers of fatal injuries.

Updating schedule:  Updates to the data become available approximately 8 months following the close 
of the reference year.


================================================================================
Section 2
================================================================================

The following CENSUS OF FATAL OCCUPATIONAL INJURIES files are on the BLS internet in the 
sub-directory pub/time.series/fw:

       fw.area			- Area codes		mapping file
       fw.case			- Case codes       	mapping file
       fw.category		- Category codes   	mapping file
       fw.category2		- Category codes   	mapping file
       fw.contacts		- Contacts for CFOI
       fw.data.0.Current	- All current year-to-date data
       fw.data.1.AllData	- All data 
       fw.datatype		- Data type codes     	mapping file
       fw.event 		- Event codes 	        mapping file
       fw.footnote 		- Footnote codes 	mapping file
       fw.industry 		- Industry codes 	mapping file
       fw.occupation 		- Occupation codes 	mapping file
       fw.series		- All series with beginning and end dates
       fw.source        	- Source codes 		mapping file	
       fw.txt			- General information
	
================================================================================
Section 3
================================================================================
The definition of a time series, its relationship to and the interrelationship
among series, data and mapping files is detailed below:

A time series refers to a set of data observed over an extended period of time
over consistent time intervals (e.g. monthly, quarterly, semi-annually, 
annually). BLS time series data are typically produced at monthly intervals and 
represent data ranging from a specific consumer item in a specific geographical 
area whose price is gathered monthly to a category of worker in a specific 
industry whose employment rate is being recorded monthly, etc.

The FTP files are organized such that data users are provided with the following
set of files to use in their efforts to interpret data files:

a)  a series file (only one series file per survey)
b)  mapping files
c)  data files

The series file contains a set of codes which, together, compose a series 
identification code that serves to uniquely identify a single time series.  
Additionally, the series file also contains the following series-level 
information:

a) the period and year corresponding to the first data observation 
b) the period and year corresponding to the most recent data observation. 

The mapping files are definition files that contain explanatory text 
descriptions that correspond to each of the various codes contained within each 
series identification code.

The data file contains one line of data for each observation period pertaining 
to a specific time series.  Each line contains a reference to the following:

a) a series identification code
b) year in which data is observed
c) period for which data is observed (A01 annual)
d) value
e) footnote code (if available)
================================================================================
Section 4
================================================================================
File Structure and Format: The following represents the file format used to 
define fw.series.  Note that the Field Numbers are for reference only; they do 
not exist in the database.  Data files are in ASCII text format.  Data elements 
are separated by tabs; the first record of each file contains the column headers 
for the data elements stored in each field.  Each record ends with a new line 
character. 

Field #/Data Element	Length		Value(Example)		

1.  series_id		  17		FWU00X00000080N00

2.  category_code*	   3		00X

3.  datatype_code*	   1		8

4.  case_code*	           1		0		
						
5.  industry_code*	   6		000000	
						
6.  event_code*		   6		0000XX

7.  source_code*	   6		0000XX

8.  occupation_code*	   6		000000

9.  area_code*		   3		N00

10.  begin_year		   4		2011	

11.  begin_period	   3		A01

12.  end_year		   4		2011		

13.  end_period		   3		A01		


*Note fields/data elements denoted with an * indicate parts of the series id that 
are also broken out separately in the fw.series file.  A more detailed breakdown of the 
series id is described below.


The series_id (FWU00X00000080N00) can be broken out into:

Code					Value

survey abbreviation 	=		FW
seasonal(code)		=		U
category_code		=		00X
	
Detailed category codes	(choose one)
industry_code*		=		000000
event_code*		=		0000XX
source_code*		=		0000XX
occupation_code*	=		000000

datatype_code		=		8
case_code		=		0
area_code		=		N00


*Please note that there are four possible detailed category codes (industry, event, source, 
and occupation) that make up the 6 character length in the 7-12 positions of the series_id.  
Any of the four can be selected, but only one at any given time. 


================================================================================
Section 5
================================================================================
File Structure and Format: The following represents the file format used to 
define each data file.  Note that the field numbers are for reference only; they 
do not exist in the database.  Data files are in ASCII text format.  Data 
elements are separated by tabs; the first record of each file contains the 
column headers for the data elements stored in each field.  Each record ends 
with a new line character. 

The fw.data file is partitioned into two separate files:  

	1.  fw.data.0.Current	- All current year-to-date data
	2.  fw.data.1.AllData	- All data

Both of the above data files have the following format:

Field #/Data Element	Length		Value(Example)		

1. series_id		  17		FWU00X00000080N00

2. year			   4		2011	

3. period		   3		A01		

4. value		  12      	4609	
				 
5. footnote_codes	  10		It varies
				

The series_id (FWU00X00000080N00) can be broken out into:

Code					Value

survey abbreviation	=		FW
seasonal(code)		=		U
category_code		=		00X

Detailed category codes	(choose one)
industry_code*		=		000000
event_code*		=		0000XX
source_code*		=		0000XX
occupation_code*	=		000000

datatype_code		=		8
case_code		=		0
area_code		=		N00

*Please note that there are four possible detailed category codes (industry, event, source, 
and occupation) that make up the 6 character length in the 7-12 positions of the series_id.  
Any of the four can be selected, but only one at any given time. 


================================================================================
Section 6
================================================================================
File Structure and Format:  The following represents the file format used to 
define each mapping file. Note that the field numbers are for reference only; 
they do not exist in the database.  Mapping files are in ASCII text format.  
Data elements are separated by tabs; the first record of each file contains the 
column headers for the data elements stored in each field.  Each record ends 
with a new line character. 


File Name:  fw.area

Field #/Data Element		Length		Value(Example)		

1.  area_code	  		3		N00

2.  area_text			100		Text

3.  display_level		2		0

4.  selectable			1		T

5.  sort_sequence		5		00001


File Name:  fw.case

Field #/Data Element		Length		Value(Example)		

1.  case_code		  	1		9

2.  case_text			100         	Text
						
				
File Name:  fw.category

Field #/Data Element		Length		Value(Example)		

1.  case_code		  	1		9

2.  category_code		3		00X		

3.  category_text		100		Text
						

File Name:  fw.category2

Field #/Data Element		Length		Value(Example)		

1.  category_code		3		00X		

2.  category_text		100		Text
						

File Name:  fw.datatype

Field #/Data Element		Length		Value(Example)		

1.  datatype_code	  	1		8

2.  datatype_text		100		Text
						

File Name:  fw.event

Field #/Data Element		Length		Value(Example)		

1.  event_code	  		6		0000XX

2.  event_text			100		Text

3.  display_level		2		0

4.  selectable			1		T

5.  sort_sequence		5		00001			


File Name:  fw.industry

Field #/Data Element		Length		Value(Example)		

1.  industry_code	  	6		000000

2.  industry_text		100		Text

3.  display_level		2		0

4.  selectable			1		T

5.  sort_sequence		5		00001


File Name:  fw.occupation

Field #/Data Element		Length		Value(Example)		

1.  occupation_code	  	6		000000

2.  occupation_text		100		Text

3.  display_level		2		0

4.  selectable			1		T

5.  sort_sequence		5		00001


File Name:  fw.source

Field #/Data Element		Length		Value(Example)		

1.  source_code	  		6		0000XX

2.  source_text			100		Text

3.  display_level		2		0

4.  selectable			1		T

5.  sort_sequence		5		00001


File Name:  fw.footnote

Field #/Data Element		Length		Value(Example)		

1.  footnote_code	  	1		P

2.  footnote_text		100		Text


================================================================================
Section 7
================================================================================
CENSUS OF FATAL OCCUPATIONAL INJURIES (FW) DATABASE ELEMENTS


Data Element	Length	    Value(Example)		Description


area_code	     3		N00		    Unique code used to identify
                                                    a specific geographic region.

area_text	     100	Text		    Describes the specific 
                                                    geographic region.

begin_year	     4		2011	            Identifies first year for 
						    which data are available.

begin_period	     3		A01		    Identifies first data 
						    observation within the first 
						    year for which data are 
						    available.

category_code	     2		00X		    The code identifies the 	
						    category of the observation.

category_text	   100		Text		    Describes the category of the
						    observation.
 
case_code	     1	      	S	            The code identifies the type 
						    of case for which data are 
						    available.

case_text	   100		Text		    Describes case for which data 
						    are available.

datatype_code	     1		 8	            Code identifying the 	
						    datatype of the observation.

datatype_text	   100		Text		    Describes the datatype of the
					            observation.

end_year	     4		2011		    Identifies last year for 
						    which data are available.

end_period	     3		A01		    Identifies last data 
						    observation within the first 
					    	    year for which data are
						    available. 

event_code	     6		0000XX		    Names the event which 
                               	    		    resulted in the reported       
	                                            injury.
						
event_text	   100		Text		    Describes the event which 
                               	    		    resulted in the reported       
	                                            injury.
 
footnote_code	     1		P		    Identifies footnote for 
						    the data series.

footnote_codes	    10		It varies	    Identifies footnotes for 
						    the data series.

footnote_text	   100		Text		    Contains the text of the 
						    footnote.

industry_code	     6		000000	            The code identifies the 	
						    industry for which data are 
						    observed.
						    
industry_text	   100          Text		    Describes the industry for 
						    which data are observed.
						    						
occupation_code	     6		000000	            The code identifies the 
						    occupation of the person     
						    experiencing the injury.

occupation_text	   100		Text		    Describes the occupation of 
						    the person experiencing the 
						    injury.

series_id	    17		FWU00X80000000N00   Code identifying the 
						    specific series.						    

source_code	     6		0000XX		    The code identifies the 
					    	    source of injury.
					    	    
source_text	   100          Text		    Describes the 
					    	    source of injury.

value		    12		Variable	    Data value for series.

year		     4		2011		    Identifies year of 
			        		    observation. 

